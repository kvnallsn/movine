//! Movine provides a library implementation for integration with codebases. This lets you easily
//! run migrations at the startup of the application.
//!
//! # Example
//! ```
//! use movine::{Movine, Config};
//! use movine::adaptor::SqliteAdaptor;
//! use movine::errors::Error;
//!
//! fn main() -> Result<(), Error> {
//!     std::env::set_var("SQLITE_FILE", ":memory:");
//!     let config = Config::load(&"movine.toml")?;
//!     let adaptor = SqliteAdaptor::from_params(&config.sqlite.unwrap())?;
//!     let mut movine = Movine::new(adaptor);
//!     /// Note: Normally you would catch the error, however due to the doc-test
//!     /// nature of this example, there is no migration directory so this command
//!     /// will fail.
//!     //movine.fix()?;
//!     movine.fix();
//!     Ok(())
//! }
//!
//! ```
use chrono::prelude::*;

#[macro_use]
extern crate log;

pub mod adaptor;
pub mod config;
mod display;
pub mod errors;
mod file_handler;
mod match_maker;
mod migration;
mod plan_builder;

use adaptor::DbAdaptor;
pub use config::Config;
use errors::{Error, Result};
use file_handler::FileHandler;
use migration::MigrationBuilder;
use plan_builder::PlanBuilder;

pub struct Movine {
    adaptor: Box<dyn DbAdaptor>,
    migration_dir: String,
    number: Option<usize>,
    show_plan: bool,
    ignore_divergent: bool,
    strict: bool,
}

impl Movine {
    pub fn new(adaptor: Box<dyn DbAdaptor>) -> Self {
        Self {
            adaptor,
            migration_dir: "./migrations".into(),
            number: None,
            show_plan: false,
            ignore_divergent: false,
            strict: false,
        }
    }

    pub fn set_migration_dir(&mut self, migration_dir: &str) -> &mut Self {
        self.migration_dir = migration_dir.into();
        self
    }

    pub fn set_number(&mut self, number: Option<usize>) -> &mut Self {
        self.number = number;
        self
    }

    pub fn set_show_plan(&mut self, show_plan: bool) -> &mut Self {
        self.show_plan = show_plan;
        self
    }

    pub fn set_ignore_divergent(&mut self, ignore_divergent: bool) -> &mut Self {
        self.ignore_divergent = ignore_divergent;
        self
    }

    pub fn set_strict(&mut self, strict: bool) -> &mut Self {
        self.strict = strict;
        self
    }

    pub fn is_initialized(&mut self) -> Result<bool> {
        self.adaptor.is_initialized()
    }

    pub fn initialize(&mut self) -> Result<()> {
        let file_handler = FileHandler::new(&self.migration_dir);
        file_handler.create_migration_directory()?;
        let up_sql = self.adaptor.init_up_sql();
        let down_sql = self.adaptor.init_down_sql();

        let init_migration = MigrationBuilder::new()
            .name(&"movine_init")
            .date(Utc.timestamp(0, 0))
            .up_sql(&up_sql)
            .down_sql(&down_sql)
            .build()?;

        match file_handler.write_migration(&init_migration) {
            Ok(_) => {}
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            x => x?,
        }

        // Can't just call to `up` function since we are unable to get
        // database migrations until we run this migration.
        let local_migrations = file_handler.load_local_migrations()?;
        let db_migrations = Vec::new();
        let plan = PlanBuilder::new()
            .local_migrations(&local_migrations)
            .db_migrations(&db_migrations)
            .up()?;
        self.adaptor.run_migration_plan(&plan)
    }

    pub fn generate(&mut self, name: &str) -> Result<()> {
        let file_handler = FileHandler::new(&self.migration_dir);
        let new_migration = MigrationBuilder::new()
            .name(name)
            .date(Utc::now())
            .build()?;
        file_handler.write_migration(&new_migration)
    }

    pub fn status(&mut self) -> Result<()> {
        let file_handler = FileHandler::new(&self.migration_dir);
        let local_migrations = file_handler.load_local_migrations()?;
        let db_migrations = self.adaptor.load_migrations()?;

        let status = PlanBuilder::new()
            .local_migrations(&local_migrations)
            .db_migrations(&db_migrations)
            .status()?;

        display::print_status(&status);
        Ok(())
    }

    pub fn up(&mut self) -> Result<()> {
        let file_handler = FileHandler::new(&self.migration_dir);
        let local_migrations = file_handler.load_local_migrations()?;
        let db_migrations = self.adaptor.load_migrations()?;

        let plan = PlanBuilder::new()
            .local_migrations(&local_migrations)
            .db_migrations(&db_migrations)
            .count(self.number)
            .set_strict(self.strict)
            .up()?;

        if self.show_plan {
            display::print_plan(&plan);
            Ok(())
        } else {
            self.adaptor.run_migration_plan(&plan)
        }
    }

    pub fn down(&mut self) -> Result<()> {
        let file_handler = FileHandler::new(&self.migration_dir);
        let local_migrations = file_handler.load_local_migrations()?;
        let db_migrations = self.adaptor.load_migrations()?;

        let plan = PlanBuilder::new()
            .local_migrations(&local_migrations)
            .db_migrations(&db_migrations)
            .count(self.number)
            .set_ignore_divergent(self.ignore_divergent)
            .down()?;

        if self.show_plan {
            display::print_plan(&plan);
            Ok(())
        } else {
            self.adaptor.run_migration_plan(&plan)
        }
    }

    pub fn fix(&mut self) -> Result<()> {
        let file_handler = FileHandler::new(&self.migration_dir);
        let local_migrations = file_handler.load_local_migrations()?;
        let db_migrations = self.adaptor.load_migrations()?;

        let plan = PlanBuilder::new()
            .local_migrations(&local_migrations)
            .db_migrations(&db_migrations)
            .fix()?;

        if self.show_plan {
            display::print_plan(&plan);
            Ok(())
        } else {
            self.adaptor.run_migration_plan(&plan)
        }
    }

    pub fn redo(&mut self) -> Result<()> {
        let file_handler = FileHandler::new(&self.migration_dir);
        let local_migrations = file_handler.load_local_migrations()?;
        let db_migrations = self.adaptor.load_migrations()?;

        let plan = PlanBuilder::new()
            .local_migrations(&local_migrations)
            .db_migrations(&db_migrations)
            .count(self.number)
            .set_ignore_divergent(self.ignore_divergent)
            .redo()?;

        if self.show_plan {
            display::print_plan(&plan);
            Ok(())
        } else {
            self.adaptor.run_migration_plan(&plan)
        }
    }
}
