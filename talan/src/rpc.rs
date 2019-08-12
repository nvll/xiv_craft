use crate::recipe;
use std::sync::mpsc::{Receiver, Sender};

pub enum Request {
    Recipe { item: String, job: u32 },
}

pub enum Response {
    Recipe(Option<recipe::Recipe>),
}

pub struct Worker {
    rx: Receiver<Request>,
    tx: Sender<Response>,
}

impl Worker {
    pub fn new(rx: Receiver<Request>, tx: Sender<Response>) -> Self {
        Worker { rx, tx }
    }

    fn receive(&self) -> Option<Request> {
        match self.rx.recv() {
            Ok(r) => Some(r),
            Err(e) => {
                log::error!("[worker rx] failed to receive request: {}", e.to_string());
                None
            }
        }
    }

    fn reply(&self, resp: Response) {
        self.tx.send(resp).unwrap_or_else(|e| {
            log::error!("[worker tx] failed to send response: {}", e.to_string())
        });
    }

    pub fn worker_thread(&self) {
        log::trace!("worker thread started");
        loop {
            if let Some(request) = self.receive() {
                match request {
                    Request::Recipe { item, job } => {
                        let recipe_result = Response::Recipe(
                            if let Ok(search_results) = xivapi::query_recipe(&item) {
                                recipe::RecipeBuilder::new(&item, job).from_results(&search_results)
                            } else {
                                None
                            },
                        );
                        self.reply(recipe_result);
                    }
                };
            }
        }
    }
}
