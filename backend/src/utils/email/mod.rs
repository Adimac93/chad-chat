use anyhow::Context;
use lettre::message::Mailbox;
use lettre::transport::smtp::{authentication::Credentials, AsyncSmtpTransport};
use lettre::{Address, AsyncTransport, Message, Tokio1Executor};
use nanoid::nanoid;

use crate::configuration::SmtpSettings;
use maud::html;

#[derive(Clone)]
pub struct Mailer(AsyncSmtpTransport<Tokio1Executor>);

impl Mailer {
    pub fn new(config: SmtpSettings) -> Self {
        Self(
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.relay)
                .unwrap()
                .credentials(config.get_credentials())
                .build(),
        )
    }

    pub async fn send_verification(&self, email: &str) -> Result<(), anyhow::Error> {
        let email = email.parse::<Address>().context("Failed to parse email")?;
        let Mailer(mailer) = self;

        let url = "https://chad-chat.up.railway.app";
        let token = nanoid!();

        let body = html! {
            h1 {"Dear chadder!"}
            p {"We are kindly greatful that you have decided to join us!"}
            p {"To proceed with maximum level of chadossity please verify your registration:"}
            a href={ (url) "/api/auth/verify?token=" (token) } {
                "Chaddify"
            }
        }
        .into_string();

        let res = mailer
            .send(
                Message::builder()
                    .from(Mailbox::new(
                        Some(String::from("Chad")),
                        Address::new("adimac93", "gmail.com").unwrap(),
                    ))
                    .to(Mailbox::new(None, email))
                    .subject("Chad chat verification")
                    .body(body)
                    .unwrap(),
            )
            .await;

        Ok(())
    }
}
