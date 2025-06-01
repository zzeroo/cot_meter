mod migrations;

use askama::Template;
use cot::{reverse_redirect, App, AppBuilder, BoxedHandler, Project};
use cot::cli::CliMetadata;
use cot::db::{Auto, Model, model};
use cot::db::migrations::SyncDynMigration;
use cot::db::query;
use cot::form::Form;
use cot::html::Html;
use cot::middleware::{AuthMiddleware, SessionMiddleware, LiveReloadMiddleware};
use cot::project::{MiddlewareContext, RegisterAppsContext};
use cot::request::extractors::Path;
use cot::request::extractors::RequestDb;
use cot::request::extractors::RequestForm;
use cot::response::Response;
use cot::router::{Route, Router, Urls};
use cot::static_files;
use cot::static_files::StaticFile;
use cot::static_files::StaticFilesMiddleware;

#[derive(Debug, Clone)]
#[model]
pub struct MeterType {
    #[model(primary_key)]
    id: Auto<i64>,
    //#[model(unique)]
    //name: LimitedString<32>,
    name: String,
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    urls: &'a Urls,
    meter_types: Vec<MeterType>,
}

async fn index(urls: Urls, RequestDb(db): RequestDb) -> cot::Result<Html> {
    let meter_types = MeterType::objects().all(&db).await?;
    let index_template = IndexTemplate {
        urls: &urls,
        meter_types
    };
    let rendered = index_template.render()?;

    Ok(Html::new(rendered))
}

#[derive(Debug, Form)]
struct MeterTypeForm {
    #[form(opt(max_length = 100))]
    name: String,
}

async fn add_meter_type(urls: Urls, RequestDb(db): RequestDb, RequestForm(meter_type_form): RequestForm<MeterTypeForm>) -> cot::Result<Response> {
    let meter_type_form = meter_type_form.unwrap();

    MeterType {
        id: Auto::auto(),
        name: meter_type_form.name,
    }.save(&db).await?;

    Ok(reverse_redirect!(urls, "index")?)
}

async fn remove_meter_type(urls: Urls, RequestDb(db): RequestDb, Path(meter_type_id): Path<i64>) -> cot::Result<Response> {
    query!(MeterType, $id == meter_type_id).delete(&db).await?;

    Ok(reverse_redirect!(urls, "index")?)
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
        Router::with_urls([
            Route::with_handler_and_name("/", index, "index"),
            Route::with_handler_and_name("/meter_types/add", add_meter_type, "add-meter-type"),
            Route::with_handler_and_name("/meter_types/{meter_type_id}/remove", remove_meter_type, "remove-meter-type"),
        ])
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
        handler: cot::project::RootHandlerBuilder,
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
