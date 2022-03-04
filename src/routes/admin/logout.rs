use actix_web::HttpResponse;

use crate::{session_state::TypedSession, utils::see_other};



pub async fn log_out(session: TypedSession) -> Result<HttpResponse, actix_web::Error> {
    session.log_out();
    
    Ok(see_other("/login"))
}