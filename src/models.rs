use crate::schema::processed_files;
use std::path::PathBuf;

use diesel::{Insertable, Queryable};

#[derive(Queryable, PartialEq, Eq, Debug)]
pub struct ProcessedFile {
    #[diesel(deserialize_as = String)]
    pub file_path: PathBuf,
}

#[derive(Insertable, PartialEq, Eq, Debug)]
#[diesel(table_name = processed_files)]
pub struct NewProcessedFile {
    pub file_path: String,
}
