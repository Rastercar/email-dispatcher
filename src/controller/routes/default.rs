use lapin::message::Delivery;

use crate::controller::router::nack_delivery;

#[tracing::instrument]
pub async fn handle_delivery_without_corresponding_rpc(delivery: Delivery) -> Result<(), String> {
    nack_delivery(&delivery).await?;
    Err("handler does not exist".to_owned())
}
