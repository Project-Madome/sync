use std::{env, fmt::Debug, str::FromStr};

use sai::{Component, ComponentLifecycle};

fn env<T>(key: &str) -> T
where
    T: FromStr,
    <T as FromStr>::Err: Debug,
{
    let var = env::var(key).expect("Please set dotenv");

    var.parse().expect("Please set dotenv to valid value")
}

#[derive(Component)]
#[lifecycle]
pub struct Config {
    per_page: Option<usize>,
}

#[async_trait::async_trait]
impl ComponentLifecycle for Config {
    async fn start(&mut self) {
        dotenv::dotenv().ok();

        self.per_page.replace(env("PER_PAGE"));
    }
}

impl Config {
    pub fn per_page(&self) -> usize {
        self.per_page.unwrap()
    }
}
