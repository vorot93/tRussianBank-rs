extern crate failure;
extern crate futures;
extern crate futures_cpupool;
extern crate google_sheets4;
extern crate hyper;
extern crate yup_oauth2;

use failure::{Error, format_err};
use futures::prelude::*;
use futures_cpupool::CpuPool;
use google_sheets4::Sheets;
use yup_oauth2::{ApplicationSecret, Authenticator, DefaultAuthenticatorDelegate, MemoryStorage, ServiceAccountKey, ServiceAccountAccess};

pub type FallibleFuture<T> = Box<Future<Item = T, Error = Error> + Send>;

#[derive(Clone, Debug)]
pub struct Status {
    pub capital: i64,
    pub borrowed: i64,
    pub reserve: i64,
    pub assets: i64,
}

pub trait Backend {
    fn status(&self) -> FallibleFuture<Status>;
}

#[derive(Clone)]
pub struct SheetsBackend {
    pub secret: ServiceAccountKey,
    pub spreadsheet_id: String,
    pub cpu_pool: CpuPool,
}

impl Backend for SheetsBackend {
    fn status(&self) -> FallibleFuture<Status> {
        let sheets = self.make_sheets();

        Box::new(self.cpu_pool.spawn_fn({
            let spreadsheet_id = self.spreadsheet_id.clone();
            move || {
                let mut values = &sheets
                    .spreadsheets()
                    .values_get(&spreadsheet_id, "B7:B10")
                    .major_dimension("COLUMNS")
                    .doit().map_err(|e| format_err!("{:?}", e))?
                    .1
                    .values.ok_or(format_err!("No data received"))?[0].clone().into_iter().map(|item| item.replace("₽", "").replace(" ", "")).collect::<Vec<_>>();

                println!("{}", values[0]);

                Ok(Status {
                    capital: values[0].parse()?,
                    borrowed: values[1].parse()?,
                    reserve: values[2].parse()?,
                    assets: values[3].parse()?,
                })
            }
        }))
    }
}

impl SheetsBackend {
    fn make_sheets(
        &self,
    ) -> Sheets<
        hyper::Client,
        ServiceAccountAccess<hyper::Client>,
    > {
        let auth = ServiceAccountAccess::new(self.secret.clone(), hyper::Client::with_connector(hyper::net::HttpsConnector::new(
            hyper_rustls::TlsClient::new(),
        )));
        Sheets::new(
            hyper::Client::with_connector(hyper::net::HttpsConnector::new(
                hyper_rustls::TlsClient::new(),
            )),
            auth,
        )
    }
}
