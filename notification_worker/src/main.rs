use dotenvy::dotenv;
use futures_lite::stream::StreamExt;
use lapin::{Connection, ConnectionProperties, options, types::FieldTable};
use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::env;
use tokio;

#[derive(Deserialize, Serialize, Debug)]
#[serde(rename_all = "snake_case")]
enum NotificationKind {
    Otp,
    Alert,
    Marketing,
}

#[derive(Deserialize, Serialize, Debug)]
struct NotificationMessage {
    user_id: i32,
    email: Option<String>,
    phone_number: Option<String>,
    destinations: Vec<String>, // e.g. ["email", "whatsapp"]
    kind: NotificationKind,
    message: Option<String>, // Optional if OTP
}

async fn handle_otp(user_id: i32, redis_conn: &mut redis::aio::MultiplexedConnection) -> String {
    let otp = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let key = format!("otp:{}", user_id);
    redis_conn
        .set_ex::<_, _, ()>(&key, &otp, 300)
        .await
        .unwrap(); // store for 5 minutes
    otp
}

async fn send_email(to: &str, content: &str) {
    println!("📧 Sending email to {}: {}", to, content);
    // TODO: Integrate lettre crate
}

async fn send_whatsapp(to: &str, content: &str) {
    println!("💬 Sending WhatsApp to {}: {}", to, content);
    // TODO: Integrate WhatsApp Cloud API
}

#[tokio::main]
async fn main() {
    dotenv().ok();
    let amqp_url = env::var("AMQP_URL").expect("AMQP_URL not set");
    let redis_url = env::var("REDIS_URL").expect("REDIS_URL not set");

    let connection = Connection::connect(&amqp_url, ConnectionProperties::default())
        .await
        .expect("Failed to connect to AMQP server");

    let channel = connection
        .create_channel()
        .await
        .expect("Failed to create channel");

    channel
        .queue_declare(
            "notifications",
            options::QueueDeclareOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("queue declare");

    let mut consumer = channel
        .basic_consume(
            "notifications",
            "worker",
            options::BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .expect("basic consume");

    let mut redis_conn = Client::open(redis_url)
        .expect("Redis client init")
        .get_multiplexed_async_connection()
        .await
        .expect("Redis connection");

    while let Some(delivery) = consumer.next().await {
        let message = delivery.expect("delivery");
        let body = message.data;
        let key = format!("notification:{}", message.delivery_tag);

        // Save raw payload to Redis
        redis_conn
            .set::<_, _, ()>(key.clone(), &body)
            .await
            .unwrap();

        // Deserialize and handle notification
        if let Ok(notification) = serde_json::from_slice::<NotificationMessage>(&body) {
            let content = match notification.kind {
                NotificationKind::Otp => {
                    Some(handle_otp(notification.user_id, &mut redis_conn).await)
                }
                _ => notification.message.clone(),
            };

            if let Some(text) = content {
                for channel in notification.destinations {
                    match channel.as_str() {
                        "email" => {
                            if let Some(email) = &notification.email {
                                send_email(email, &text).await;
                            }
                        }
                        "whatsapp" => {
                            if let Some(phone) = &notification.phone_number {
                                send_whatsapp(phone, &text).await;
                            }
                        }
                        _ => println!("Unknown destination: {}", channel),
                    }
                }
            }
        } else {
            println!("❌ Failed to parse notification message: {:?}", body);
        }

        channel
            .basic_ack(message.delivery_tag, options::BasicAckOptions::default())
            .await
            .expect("ack");
    }
}
