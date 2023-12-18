use anyhow::Result;
use crossbeam_channel::Receiver;
use std::{path::PathBuf, sync::Arc};

use dashmap::DashSet;
use threadpool::ThreadPool;

pub struct FileProcessingManager {
    pool: ThreadPool,
    work_queue: Receiver<(Arc<PathBuf>, Arc<PathBuf>)>,
    shared_resource: Arc<FileProcessingResource>,
}

type FileProcessHandler = Box<dyn Fn(&Arc<PathBuf>, &Arc<PathBuf>) -> Result<bool> + Sync + Send>;
type FileProcessResultHandler = Box<dyn Fn(&Arc<PathBuf>, Result<bool>) + Sync + Send>;

struct FileProcessingResource {
    in_process: Arc<DashSet<Arc<String>>>,
    handler: FileProcessHandler,
    result_handler: FileProcessResultHandler,
}

impl FileProcessingManager {
    pub fn new(
        n_thread: usize,
        recv: Receiver<(Arc<PathBuf>, Arc<PathBuf>)>,
        handler: FileProcessHandler,
        result_handler: FileProcessResultHandler,
    ) -> FileProcessingManager {
        FileProcessingManager {
            pool: ThreadPool::new(n_thread),
            work_queue: recv,
            shared_resource: Arc::new(FileProcessingResource {
                in_process: Arc::new(DashSet::new()),
                handler,
                result_handler,
            }),
        }
    }

    pub fn process_once(&self) -> Result<()> {
        let (file, drop_src_dir) = self.work_queue.recv()?;
        let cloned_resource = self.shared_resource.clone();
        self.pool
            .execute(move || cloned_resource.process_file(file, drop_src_dir));
        Ok(())
    }
}

impl FileProcessingResource {
    fn process_file(&self, file: Arc<PathBuf>, drop_src_dir: Arc<PathBuf>) {
        let file_string = Arc::new(file.display().to_string());
        if self.in_process.insert(file_string.clone()) {
            let result = (self.handler)(&file, &drop_src_dir);
            (self.result_handler)(&file, result);
            self.in_process.remove(&file_string);
        }
    }
}
