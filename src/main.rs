mod migrations;

use askama::Template;
use cot::cli::CliMetadata;
use cot::db::migrations::SyncDynMigration;
use cot::html::Html;
use cot::middleware::{AuthMiddleware, LiveReloadMiddleware, SessionMiddleware};
use cot::project::{MiddlewareContext, RegisterAppsContext, RootHandlerBuilder};
use cot::router::{Route, Router};
use cot::static_files::{StaticFile, StaticFilesMiddleware};
use cot::{App, AppBuilder, BoxedHandler, Project, static_files};

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct IndexTemplate {}

async fn index() -> cot::Result<Html> {
    let index_template = IndexTemplate {};
    let rendered = index_template.render()?;

    Ok(Html::new(rendered))
}

struct CotMeterApp;

// An app is a collection of views and other components that make up a part of your service.
impl App for CotMeterApp {
    fn name(&self) -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn migrations(&self) -> Vec<Box<SyncDynMigration>> {
        cot::db::migrations::wrap_migrations(migrations::MIGRATIONS)
    }

    fn router(&self) -> Router {
        Router::with_urls([Route::with_handler_and_name("/", index, "index")])
    }

    fn static_files(&self) -> Vec<StaticFile> {
        static_files!("css/main.css")
    }
}

struct CotMeterProject;

// A project is a collection of apps, middlewares, and other components that make up your service.
impl Project for CotMeterProject {
    fn cli_metadata(&self) -> CliMetadata {
        cot::cli::metadata!()
    }

    fn register_apps(&self, apps: &mut AppBuilder, _context: &RegisterAppsContext) {
        apps.register_with_views(CotMeterApp, "");
    }

    fn middlewares(
        &self,
        handler: RootHandlerBuilder,
        context: &MiddlewareContext,
    ) -> BoxedHandler {
        handler
            .middleware(StaticFilesMiddleware::from_context(context))
            .middleware(AuthMiddleware::new())
            .middleware(SessionMiddleware::new())
            .middleware(LiveReloadMiddleware::from_context(context))
            .build()
    }
}

#[cot::main]
fn main() -> impl Project {
    CotMeterProject
}
