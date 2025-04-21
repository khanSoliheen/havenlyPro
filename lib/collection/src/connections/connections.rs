use lapin::{Channel, Connection, ConnectionProperties};
use redis::{Client as RedisClient, aio::MultiplexedConnection};
use std::env;

pub async fn create_amqp_channel() -> Channel {
    let amqp_url = env::var("AMQP_URL").expect("AMQP_URL not set");
    let connection = Connection::connect(&amqp_url, ConnectionProperties::default())
        .await
        .expect("Failed to connect to RabbitMQ");
    connection
        .create_channel()
        .await
        .expect("Create channel failed")
}

pub async fn create_redis_conn() -> MultiplexedConnection {
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL not set");
    RedisClient::open(redis_url)
        .expect("Failed to create Redis client")
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis")
}
