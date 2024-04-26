use axum::response::ErrorResponse;
use axum::Form;
async fn register(Form(signup): Form<Register>) -> Result<(), ErrorResponse> {
    todo!()
}

pub struct Register {
    email: String,
    username: String,
    password: String,
}
