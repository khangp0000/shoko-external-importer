-- Your SQL goes here
CREATE TABLE processed_files (file_path VARCHAR PRIMARY KEY NOT NULL ON CONFLICT ROLLBACK);