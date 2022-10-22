use sai::Component;
use tokio::sync::mpsc;

#[derive(Component)]
pub struct Channel {
    id: Option<mpsc::Sender<u32>>,
    err: Option<mpsc::Sender<crate::Error>>,
}

impl Channel {
    pub fn id(&self) -> mpsc::Sender<u32> {
        self.id.clone().unwrap()
    }

    pub fn err(&self) -> mpsc::Sender<crate::Error> {
        self.err.clone().unwrap()
    }
}
