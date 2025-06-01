mod migrations;

use askama::Template;
use async_trait::async_trait;
use cot::{reverse_redirect, App, AppBuilder, BoxedHandler, Project};
use cot::admin::{AdminApp, DefaultAdminModelManager};
use cot::admin::AdminModel;
use cot::auth::db::{DatabaseUser, DatabaseUserApp};
use cot::cli::CliMetadata;
use cot::common_types::Password;
use cot::db::{Auto, Model, model};
use cot::db::migrations::SyncDynMigration;
use cot::db::query;
use cot::form::Form;
use cot::html::Html;
use cot::middleware::{AuthMiddleware, SessionMiddleware, LiveReloadMiddleware};
use cot::project::{MiddlewareContext, RegisterAppsContext};
use cot::ProjectContext;
use cot::request::extractors::{Path, RequestDb, RequestForm};
use cot::response::Response;
use cot::router::{Route, Router, Urls};
use cot::static_files;
use cot::static_files::{StaticFile, StaticFilesMiddleware};
use std::env;
use std::fmt::Display;

#[derive(Debug, Clone, Form, AdminModel)]
#[model]
pub struct MeterType {
    #[model(primary_key)]
    id: Auto<i64>,
    //#[model(unique)]
    //name: LimitedString<32>,
    name: String,
}

impl Display for MeterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
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
#[async_trait]
impl App for CotMeterApp {
    fn name(&self) -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn admin_model_managers(&self) -> Vec<Box<dyn cot::admin::AdminModelManager>> {
        vec![Box::new(DefaultAdminModelManager::<MeterType>::new())]
    }

    async fn init(&self, context: &mut ProjectContext) -> cot::Result<()> {
        // Check if admin user exists
        let admin_user_name = env::var("ADMIN_USER").unwrap_or_else(|_| "admin".to_string());
        let user = DatabaseUser::get_by_username(context.database(), "admin").await?;
        if user.is_none() {
            let password = env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "change me".to_string());
            // Create admin user
            DatabaseUser::create_user(context.database(), &admin_user_name, &Password::new(&password)).await?;
        }

        Ok(())
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
        apps.register(DatabaseUserApp::new()); // Needed for admin authentication
        apps.register_with_views(AdminApp::new(), "/admin"); // Register the admin app
        apps.register_with_views(CotMeterApp, "");
    }

    fn middlewares(
        &self,
        handler: cot::project::RootHandlerBuilder,
        context: &MiddlewareContext,
    ) -> BoxedHandler {
        handler
            .middleware(StaticFilesMiddleware::from_context(context))
            .middleware(SessionMiddleware::new()) // Required for admin login
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
