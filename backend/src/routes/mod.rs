mod escrows;
mod webhooks;
mod stats;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v1")
            .service(escrows::get_escrows)
            .service(escrows::get_escrow_by_address)
            .service(webhooks::create_webhook)
            .service(webhooks::delete_webhook)
            .service(stats::get_stats),
    );
}
