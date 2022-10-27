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

use log::LevelFilter;
use log4rs_date_appender as log4rs;
use sai::System;
use sync::RootRegistry;
use tokio::signal::{self, unix::SignalKind};

#[derive(Debug)]
struct Date;

impl log4rs::CurrentDate for Date {}

#[tokio::main]
async fn main() {
    println!("Hello, world!");

    log4rs::init_config(log4rs::config::<Date>(
        "log/{year}-{month}-{day}.log",
        LevelFilter::Debug,
    ));

    let mut system = System::<RootRegistry>::new();

    system.start().await;

    let mut sigterm = signal::unix::signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = sigterm.recv() => {},
        _ = async { signal::ctrl_c().await.expect("failed to listen for ctrl_c event") } => {}
    };

    system.stop().await;

    log::info!("gracefully shutdown the app");

    // let madome_url = "https://test.api.madome.app";

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

    // 이미지 서버에서 어떤 작품의 이미지가 얼마나 있는지 뭐가 있는지 가져올 수 있음
    // 가져와서 이걸로 뭘 할 거냐
    // 실패한 작품들 이미지 이어받기

    // 실패한 작품을 다시 받는 건 수동으로 해야함
    //

    // TODO: 토큰 최초 발급을 대화형으로 만들자
    // TODO: library 서버에 ws 구현
    // TODO: madome health check (loop로 감싼 다음 주기적으로 마도메 헬스체크해서 일정 횟수이상 안되면 꺼지게 tokio::select에 넣자)
    // 근데 그렇게 하면 내부 동작도 멈춰야하는데
    // TODO: error handle
    // TODO: 새 파일서버를 사용하게 된다면 madome-sdk에서 file api의 url을 수정해야함
    // TODO: 로그 파일에서 에러를 찾아내는 유틸
    // COMP: 각 container의 로직을 thread를 생성해서 백그라운드로 돌아가게 하기
    //

    // add_book에서는 pre-release임을 저장
    // sync에서 해당 작품의 모든 이미지 업로드에 성공했으면
    // library에 release 요청을 날림

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
