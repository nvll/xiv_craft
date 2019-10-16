use crate::config;
use crate::craft;
use crate::macros;
use crate::recipe;
use crate::task;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug)]
pub enum Request {
    Recipe {
        item: String,
        job: Option<u32>,
        count: u32,
    },
    Craft {
        options: config::Options,
        tasks: Vec<task::Task>,
        macros: Vec<macros::MacroFile>,
    },
    StopCrafting,
}

#[derive(Debug)]
pub enum Response {
    Recipe {
        recipe: Option<recipe::Recipe>,
        count: u32,
    },
    Craft(Vec<task::Status>),
    EOW, // End of Work, aka finished.
}

pub struct Worker {
    rx: Receiver<Request>,
    tx: Sender<Response>,
}

impl Worker {
    pub fn new(rx: Receiver<Request>, tx: Sender<Response>) -> Self {
        Worker { rx, tx }
    }

    fn try_receive(&self) -> Option<Request> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            _ => None,
        }
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
                    Request::Recipe { item, job, count } => {
                        log::trace!("querying xivapi for \"{}\" (job: {:?})", item, job);
                        let recipe_result = if let Ok(search_results) = xivapi::query_recipe(&item)
                        {
                            recipe::Recipe::filter(&search_results, &item, job)
                        } else {
                            None
                        };
                        log::trace!("query result: {:#?}", recipe_result);
                        self.reply(Response::Recipe {
                            recipe: recipe_result,
                            count,
                        });
                    }
                    Request::Craft {
                        options,
                        tasks,
                        macros,
                    } => {
                        // Send a full status update to the main thread after completing
                        // an item.
                        let status_fn = |status: &[task::Status]| {
                            self.reply(Response::Craft(status.to_vec()));
                        };

                        // Check whether crafting should continue after each craft.
                        let continue_fn = || -> bool {
                            if let Some(r) = self.try_receive() {
                                if let Request::StopCrafting = r {
                                    return false;
                                }
                            }
                            true
                        };

                        // If init throws an error we'll have a log to console anyway.
                        if let Ok(handle) = xiv::init() {
                            craft::craft_items(
                                handle,
                                &options,
                                &tasks[..],
                                &macros[..],
                                status_fn,
                                continue_fn,
                            );
                        }
                        self.reply(Response::EOW);
                    }
                    unknown => log::error!("Unexpected RPC received: {:?}", unknown),
                };
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use failure::Error;
    use std::sync::mpsc::{channel, Receiver, Sender};
    use std::thread;

    fn setup() -> (Sender<Request>, Receiver<Response>) {
        let (client_tx, worker_rx): (Sender<Request>, Receiver<Request>) = channel();
        let (worker_tx, client_rx): (Sender<Response>, Receiver<Response>) = channel();
        thread::spawn(move || Worker::new(worker_rx, worker_tx).worker_thread());

        (client_tx, client_rx)
    }

    // Test that a simple Request -> Response cycle works with the worker thread
    // in the background using a sample recipe. This tests both having the job
    // specified and not.
    #[test]
    fn worker_recipe_test() -> Result<(), Error> {
        let (tx, rx) = setup();
        let item1 = "Rakshasa Axe";
        tx.send(Request::Recipe {
            item: item1.to_string(),
            job: None,
            count: 1,
        })?;

        match rx.recv()? {
            Response::Recipe { recipe, count } => {
                let r = recipe.unwrap();
                assert!(r.name == item1);
                assert!(r.job == 1); // BSM
                assert!(r.index == 0);
                assert!(count == 1);
            }
            _ => panic!("unexpected response"),
        }

        let item2 = "Cloud Pearl";
        tx.send(Request::Recipe {
            item: item2.to_string(),
            job: Some(5),
            count: 3,
        })?;

        match rx.recv()? {
            Response::Recipe { recipe, count } => {
                let r = recipe.unwrap();
                assert!(r.name == item2);
                assert!(r.job == 5); // WVR
                assert!(r.index == 11);
                assert!(count == 3);
            }
            _ => panic!("unexpected response"),
        }
        Ok(())
    }

    // Ensure that we can queue up recipes + count tuples to the worker and
    // receive them on the other end in the proper order and count to create a
    // task list.
    #[test]
    fn worker_recipe_list_test() -> Result<(), Error> {
        let (tx, rx) = setup();
        let recipe_list = vec![
            ("Cloud Pearl", 1),
            ("Prismatic Ingot", 2),
            ("Rakshasa Axe", 3),
            ("White Ash Lumber", 1),
        ];
        let mut tasks: Vec<task::Task> = Vec::new();
        for (item, count) in &recipe_list {
            tx.send(Request::Recipe {
                item: item.to_string(),
                count: *count,
                job: None,
            })
            .unwrap();
        }

        for _ in 0..recipe_list.len() {
            let resp = rx.recv().unwrap();
            match resp {
                Response::Recipe { recipe, count } => {
                    tasks.push(task::Task::new(recipe.unwrap(), count));
                }
                _ => panic!("unexpected response"),
            }
        }

        for i in 0..recipe_list.len() {
            assert_eq!(recipe_list[i].0, tasks[i].recipe.name);
            assert_eq!(recipe_list[i].1, tasks[i].quantity);
        }

        Ok(())
    }
}
