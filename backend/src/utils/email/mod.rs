use crate::configuration::SmtpSettings;
use anyhow::Context;
use lettre::message::Mailbox;
use lettre::transport::smtp::response::Response;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::transport::smtp::Error;

use lettre::{Address, AsyncTransport, Message, Tokio1Executor};
use maud::html;
use nanoid::nanoid;
use uuid::Uuid;

#[derive(Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    origin: String,
}

impl Mailer {
    pub fn new(config: SmtpSettings, origin: String) -> Self {
        Self {
            transport: AsyncSmtpTransport::<Tokio1Executor>::relay(&config.relay)
                .unwrap()
                .credentials(config.get_credentials())
                .build(),
            origin,
        }
    }

    async fn send_mail(
        &self,
        email: Mailbox,
        subject: &str,
        body: String,
    ) -> Result<Response, Error> {
        let res = self
            .transport
            .send(
                Message::builder()
                    .from(Mailbox::new(
                        Some(String::from("Chad")),
                        Address::new("adimac93", "gmail.com").unwrap(),
                    ))
                    .to(email)
                    .subject(subject)
                    .body(body)
                    .unwrap(),
            )
            .await?;

        Ok(res)
    }

    pub async fn send_verification(
        &self,
        email: &str,
        token_id: &Uuid,
    ) -> Result<(), anyhow::Error> {
        let email = email.parse::<Address>().context("Failed to parse email")?;

        let url = "https://chad-chat.up.railway.app";

        let body = html! {
            h1 {"Dear chadder!"}
            p {"We are kindly grateful that you have decided to join us!"}
            p {"To proceed with maximum level of chadossity please verify your registration:"}
            a href={ (self.origin) "/api/auth/verify/registration?token=" (token_id.to_string()) } {
                "Chaddify"
            }
        }
        .into_string();

        self.send_mail(Mailbox::new(None, email), "Chad chat verification", body)
            .await?;

        Ok(())
    }
}
