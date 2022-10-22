// async fn get_ids(queue: Sender<u32>) {
// 해당 작품이 있는지 없는지 어떻게 알아낼 것인가?
//
// 1. 썸네일 올리고
// 2. 이미지를 다 올리고
// 3. 작품 정보를 올리자
// 이렇게 하면
// 이미지가 없으면 책 정보도 안 올라옴
//
// library 서버에서 애초에 검증을 할까?
// 작품 정보를 업로드 하는데 파일 서버에서 이미지 갯수가 안 맞으면 실패하도록할까
//
// 없는 이미지만 받을 수 있도록 하자
//
// 작품번호를 제목으로 빈 파일을 만들어서 대기열을 만들자
// 이렇게 하면 현재 작업 중인 건 따로 다른 폴더로 보내야지 (대기열, 작업 중, 작업 완료?)
// 트랜잭션을 완벽하게 구현해야 함
//
// 작품 정보 업데이트는 작품 서버에서 크롤러로 작품의 해쉬값을 보내서 비교하는 방식으로 할 건데,
// 한 달 이상 업데이트 확인을 하지 않은 작품들에 대해서 업데이트를 하자
// }

// 실시간 상태 업데이트
// 작품 별로 어디까지 진행됐는지
// download->upload를 진행 중이면 몇 페이지 중에 몇 페이지까지 됐는지 (db를 안 쓰고 할 수 있으면 제일 좋을 듯)
//
// 작품번호 폴더
// - status 파일
// - progress 파일
// - error_{timestamp} 파일

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    let madome_url = "https://test.api.madome.app";

    // 마도메 헬스체크를 통해 마도메와의 연결이 원활하지 않을 경우에는 앱 자체를 멈춰야 함
    // 실패한 작업은 진행 중인 작업이 없을 경우에만 시도함
    // TODO: 실패했을 시에는 해당 container를 명시하고 저장해야함
    // token manage container
    // 이미지 다운로드 container
    //  이미지 다운로드에서 실패한다면, 현재 이미지들의 정보를 저장하고, 어디까지 진행했는지 저장하고, 다음에 시도할 때 이미지 정보를 새로 받아와서 이전 이미지 정보와 비교 후 다르다면 처음부터 다시 시작하고 같다면 실패 시점부터 다시 시작함
    // 작품 id parse container
    // 작품 정보 다운로드 container
    // 업로드 container (이미지, 작품 정보 등 마도메에 업로드 하는 것들)
    // 작품 정보 업데이트 container (주기적으로 한달에 한 번?)
    // community container (container<->container 통신) mpsc, receiver를 사용하는 쪽에서 sender를 생성해서 갱신해주어야함 community.update_id_sender(tx);

    // 히토미에서 전체 작품을 훑어서 마도메에 없는 작품이 있을 수도 있으니까 이것도 받아줘야함

    /* let state = Arc::new(Mutex::new(State::new()));
    let (id_tx, mut id_rx) = mpsc::channel::<u32>(128);
    let (err_tx, err_rx) = mpsc::channel::<sync::Error>(16);

    let a = {
        let token = token.clone();
        // let state = state.clone();
        let err_tx = err_tx.clone();
        // let id_tx = id_tx.clone();

        tokio::spawn(async move {
            let mut empty_count = 0;
            let mut ids_store = Vec::new();

            loop {
                let mut state = state.lock().await;
                let ids = none_to_continue!(
                    crawler::nozomi::parse(state.next_page(), state.per_page())
                        .to(err_tx.clone())
                        .await
                );

                // 작품 서버에 보내서 없는 작품 id를 받아내기

                let a = none_to_continue!(
                    library::get_books_by_ids(madome_url, &token, ids.clone())
                        .to(err_tx.clone())
                        .await
                )
                .into_iter()
                .map(|x| x.id)
                .collect::<Vec<_>>();

                let mut ids = ids
                    .into_iter()
                    .filter(|x| !a.contains(x))
                    .collect::<Vec<_>>();

                if ids.is_empty() {
                    empty_count += 1;
                } else {
                    ids_store.append(&mut ids);
                    /* for id in ids {
                        id_tx.send(id).await.unwrap();
                    } */
                }

                // 현재 page를 언제 어떻게 초기화할지
                // 작품이 없는 사이클 +3 page가 되면 초기화

                if empty_count >= 3 {
                    // asc
                    ids_store.sort();

                    for id in ids_store {
                        id_tx.send(id).await.unwrap();
                    }

                    ids_store = Vec::new();

                    state.clear();
                    sleep(Duration::from_secs(300)).await;
                }
            }
        })
    };

    let b = {
        // let err_tx = err_tx.clone();

        tokio::spawn(async move {
            'a: while let Some(id) = id_rx.recv().await {
                use crawler::image::{Image, ImageKind};

                let gallery =
                    none_to_continue!(crawler::gallery::parse(id).to(err_tx.clone()).await);
                let total_page = gallery.files.len();

                let images = gallery
                    .files
                    .iter()
                    .map(|f| Image::new(id, f, ImageKind::Original))
                    .enumerate();

                for (page, fut) in images {
                    log::info!("{id}; download->upload; {page} / {total_page}");

                    let image = fut.await;

                    let buf =
                        none_to_continue!(image.download().to(err_tx.clone()).await, 'a).to_vec();

                    let path = format!("image/library/{}/{}.{}", id, page, image.ext());

                    // 이미지 업로드
                    none_to_continue!(
                        file::upload("https://file.madome.app", &token, path, &buf)
                            .to(err_tx.clone())
                            .await,
                        'a
                    );
                }

                {
                    let Gallery {
                        id,
                        title,
                        kind,
                        files,
                        language,
                        tags,
                        date,
                    } = gallery;

                    let tags = tags
                        .into_iter()
                        .map(|t| (t.kind.to_string(), t.name))
                        .collect::<Vec<_>>();

                    log::info!("{id} - Information Upload");

                    // 작품 정보 업로드
                    none_to_continue!(
                        library::add_book(
                            madome_url,
                            &token,
                            id,
                            title,
                            kind,
                            files.len(),
                            language.unwrap_or_default(),
                            date,
                            tags,
                        )
                        .to(err_tx.clone())
                        .await
                    );
                }
            }
        })
    }; */

    // TODO: thread health check
    // TODO: error handle

    // TODO: 작품 정보, 이미지 다운로드 등 종류에 따라 대기열 수를 따로 정해야하기 때문에, channel도 따로 두는 게 나을 것 같음
    /* let (task_tx, task_rx) = mpsc::channel::<u32>(1024);
    let (err_tx, err_rx) = mpsc::channel::<sync::Error>(6);
    let state = Arc::new(RwLock::new(State::new()));

    async fn add_task(state: Arc<RwLock<State>>, task_tx: Sender<u32>) -> Result<(), sync::Error> {
        let (page, per_page) = {
            let x = state.read();
            (x.page, x.per_page)
        };

        let ids = crawler::nozomi::parse(page, per_page).await?;

        for id in ids {
            task_tx.send(id).await.unwrap();
        }

        Ok(())
    }

    async fn recv_task(task_rx: &mut Receiver<u32>) -> Option<u32> {
        task_rx.recv().await
    }

    let a = {
        let state = state.clone();

        tokio::spawn(async move {
            loop {
                add_task(state.clone(), task_tx.clone()).await?;
            }

            #[allow(unreachable_code)]
            Ok::<(), sync::Error>(())
        })
    };

    tokio::select! {
        _ = a => {

        }
    } */

    /* let add_task_to_queue = {
        let task_tx = task_tx.clone();
        let token = token.clone();

        // 에러를 err_tx로 보내야함
        // 에러가 나면 죽는데, 다시 살리거나 해야함
        // 다시 살릴 경우 일정 시간 후에 작동한다거나 그런 장치가 있어야할듯?
        // 그리고 대시보드에서 해당 기능을 사용할 수 있기 때문에 어떤 메세지를 받으면 하도록 해야겠음
        // 일정 시간 후에 하는 건 아주 최상위 코드에서만
        // 이런 세세한 기능들은 트리거처럼 작동해야함
        tokio::spawn(async move {
            loop {
                // TODO: state도 init하고 reset하고 다음 페이지로 넘어간다던가 그런 게 필요할듯
                // 변수의 위치 조정도 필수

                // TODO: 마도메에 있는지 없는지 확인하고 필터링
                // 그리고 다음 페이지도 검색
                // 현재 세션을 끝내는 건 기준을 어떻게 할 지 고민
            }

            #[allow(unreachable_code)]
            Ok::<(), sync::Error>(())
        })
    }; */

    // add task
    // recv task -> parse book info -> push book info (대시보드에서 책 정보를 보는 게 우선) -> get images info (이렇게 따로 저장해야되는 정보는 대시보드에서 즉각적으로 받아오는 게 좋을 듯) -> add images

    // 에러 핸들링 필수
    // mpsc channel로 넘겨줘야함
    // main함수를 하나 만든 다음에 wrapping 시키자

    // update 해야하는 작품을 hashing을 해서 구분하자

    // 이미지를 받을 때, 파일 서버에서 이미지 목록을 먼저 가져오고, 가져온 이미지 목록에서 없는 것만 업로드하기
    // 생각해야할게 많다
    // 옛날에 업로드한 이미지들은 이름이 페이지 순서대로가 아니라 이상하게 되어 있을 것 같거든

    // 1. 섬네일 먼저 업로드하고
    // 2. 작품 정보 업로드
    // 3. 그리고 작품 이미지들 업로드

    // mpsc channel을 이용해서 여러가지 실시간 정보를 받아오면 될 듯
    // 작업 종류 단위로 함수를 만들어서
    // 메인 함수에서 반복문을 돌리면 될 듯 rayon 같은

    // 대시보드를 만들고 싶다
    // - 작품 하나씩 현황을 볼 수 있어야함 (성공 여부, 진행 중이면 뭘 진행 중인지, 실패 했다면 왜 실패했는지) enum으로 channel하나 만들어서 통신
    // - 작품 하나씩 컨트롤 가능해야함 (이건 좀 에반듯, 작업 하나마다 통신 수단을 열어놓고 제어하는 게 아닌 이상 하기 힘들 거 같음)
    // - 현재 실행 중인 인스턴스의 설정을 변경할 수 있어야함
    // - 현재 작업 중인 내역 그리고 작업했던 내역을 볼 수 있어야함
    // - 현재 사이클을 중단시킬 수 있어야함

    // 1. get ids from nozomi
    // 2. check don't have
    // 3.
}
