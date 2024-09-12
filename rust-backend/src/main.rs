use actix_web::{get, post, put, delete, web, App, HttpServer, HttpResponse, Responder};
use actix_cors::Cors;
use sqlx::PgPool;
use dotenv::dotenv;
use std::env;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, sqlx::FromRow)]
struct Task {
    id: i32,
    title: String,
    description: Option<String>,
    completed: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct CreateTask {
    title: String,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateTask {
    title: Option<String>,
    description: Option<String>,
    completed: Option<bool>,
}

#[get("/health_check")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("Server is running")
}

#[get("/tasks")]
async fn get_tasks(pool: web::Data<PgPool>) -> Result<HttpResponse, actix_web::Error> {
    let tasks = sqlx::query_as!(Task, "SELECT * FROM tasks")
        .fetch_all(pool.get_ref())
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;
    Ok(HttpResponse::Ok().json(tasks))
}

#[post("/tasks")]
async fn create_task(pool: web::Data<PgPool>, new_task: web::Json<CreateTask>) -> Result<HttpResponse, actix_web::Error> {
    let task = sqlx::query_as!(
        Task,
        r#"
        INSERT INTO tasks (title, description)
        VALUES ($1, $2)
        RETURNING id, title, description, completed
        "#,
        new_task.title,
        new_task.description
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Database error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Database error")
    })?;

    Ok(HttpResponse::Created().json(task))
}

#[put("/tasks/{id}")]
async fn update_task(
    pool: web::Data<PgPool>,
    task_id: web::Path<i32>,
    updated_task: web::Json<UpdateTask>
) -> Result<HttpResponse, actix_web::Error> {
    let task = sqlx::query_as!(
        Task,
        r#"
        UPDATE tasks
        SET title = COALESCE($1, title),
            description = COALESCE($2, description),
            completed = COALESCE($3, completed)
        WHERE id = $4
        RETURNING id, title, description, completed
        "#,
        updated_task.title,
        updated_task.description,
        updated_task.completed,
        task_id.into_inner()
    )
    .fetch_one(pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("Database error: {:?}", e);
        actix_web::error::ErrorInternalServerError("Database error")
    })?;

    Ok(HttpResponse::Ok().json(task))
}

#[delete("/tasks/{id}")]
async fn delete_task(pool: web::Data<PgPool>, task_id: web::Path<i32>) -> Result<HttpResponse, actix_web::Error> {
    let result = sqlx::query!("DELETE FROM tasks WHERE id = $1", task_id.into_inner())
        .execute(pool.get_ref())
        .await
        .map_err(|e| {
            eprintln!("Database error: {:?}", e);
            actix_web::error::ErrorInternalServerError("Database error")
        })?;

    if result.rows_affected() == 0 {
        Ok(HttpResponse::NotFound().finish())
    } else {
        Ok(HttpResponse::NoContent().finish())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await.expect("Failed to create pool");

    println!("Starting server at http://127.0.0.1:8080");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header();

        App::new()
            .wrap(cors)
            .app_data(web::Data::new(pool.clone()))
            .service(health_check)
            .service(get_tasks)
            .service(create_task)
            .service(update_task)
            .service(delete_task)
    })    
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}