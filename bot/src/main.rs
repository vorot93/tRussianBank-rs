extern crate failure;
extern crate futures;
extern crate telebot;
extern crate tokio_core;
extern crate futures_cpupool;
extern crate sheets_backend;
extern crate serde_json;
extern crate yup_oauth2;

use futures::prelude::*;
use futures_cpupool::CpuPool;
use sheets_backend::{Backend, SheetsBackend};
use std::sync::Arc;
use telebot::{bot, objects};
use tokio_core::reactor::Core;

#[derive(Copy, Clone, Debug)]
pub enum Command {
    Status,
    Search,
    Debts,
    Cards,
    Proxy,
    DuckList,
    Rules,
    AboutMe,
    AboutMyPayment,
}

impl Command {
    fn cmd(&self) -> &'static str {
        match self {
            Command::Status => "/status",
            _ => unimplemented!(),
        }
    }
}

pub struct BotController {
    pub message: objects::Message,
    pub storage: Arc<Backend>,
}

impl BotController {
    pub fn status(&self) -> Box<Future<Item = String, Error = failure::Error>> {
        Box::new(self.storage.status().map(|status| format!("{:?}", status)))
    }
}

fn main() {
    let mut lp = Core::new().unwrap();

    let cpu_pool = CpuPool::new_num_cpus();
    let secret = yup_oauth2::service_account_key_from_file(&std::env::var("SECRETS_FILE").expect("Secrets file location not specified")).unwrap();
    let spreadsheet_id = std::env::var("SPREADSHEET_ID").expect("Spreadsheet ID not specified");

    let backend = Arc::new(SheetsBackend { secret, spreadsheet_id, cpu_pool }) as Arc<Backend>;

    let token = std::env::var("TELEGRAM_BOT_TOKEN").expect("Bot token not specified");
    let bot = bot::RcBot::new(lp.handle(), &token).update_interval(200);

    let controller_factory = Arc::new(move |_bot, message| BotController { storage: backend.clone(), message });

    for (cmd, f) in &[(Command::Status, BotController::status)] {
        let controller_factory = controller_factory.clone();
        let handle = bot
            .new_cmd(cmd.cmd())
            .and_then(move |(bot, message)| {
                println!("Received request: {:?}", message);
                f(&controller_factory(bot, message))
            });
        bot.register(handle);
    }

    bot.run(&mut lp).unwrap();
}
