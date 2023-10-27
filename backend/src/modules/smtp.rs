use crate::configuration::SmtpSettings;
use anyhow::Context;
use lettre::message::Mailbox;
use lettre::message::MultiPart;
use lettre::transport::smtp::response::Response;
use lettre::transport::smtp::AsyncSmtpTransport;
use lettre::transport::smtp::Error;
use lettre::{Address, AsyncTransport, Message, Tokio1Executor};
use maud::html;
use uuid::Uuid;

#[derive(Clone)]
pub struct Mailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    origin: String,
    address: Address,
}

impl Mailer {
    pub fn new(config: SmtpSettings, origin: String) -> Self {
        Self {
            transport: AsyncSmtpTransport::<Tokio1Executor>::relay(&config.relay)
                .unwrap()
                .credentials(config.get_credentials())
                .build(),
            origin,
            address: config.get_address(),
        }
    }

    async fn send_mail(
        &self,
        email: Mailbox,
        subject: &str,
        multipart: MultiPart,
    ) -> Result<Response, Error> {
        let res = self
            .transport
            .send(
                Message::builder()
                    .from(Mailbox::new(
                        Some(String::from("Chad")),
                        self.address.clone(),
                    ))
                    .to(email)
                    .subject(subject)
                    .multipart(multipart)
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

        let body = html! {
            h1 {"Dear chadder!"}
            p {"We are kindly grateful that you have decided to join us!"}
            p {"To proceed with maximum level of chadossity please verify your registration:"}
            a href={ (self.origin) "/api/auth/verify/registration?token=" (token_id.to_string()) } {
                "Chaddify"
            }
        }
        .into_string();

        self.send_mail(
            Mailbox::new(None, email),
            "Chad chat verification",
            MultiPart::alternative_plain_html(body.to_string(), body),
        )
        .await?;

        Ok(())
    }

    // pub async fn send_new_ip_notification(
    //     &self,
    //     email: &str,
    //     token_id: &Uuid,
    // ) -> Result<(), anyhow::Error> {
    //     let token = "a";
    //     let body = html! {
    //         h1 {"Be careful chadder!"}
    //         p {"New IP emerged beneath our chad network!"}
    //         p {"To ensure your account security belongs only to true chad decide if this IP should be trusted by chadnet Inc."}
    //         a href={ (self.origin) "/api/auth/verify/ip?token=" (token_id.to_string()) } {
    //             "Chaddify"
    //         }
    //     }
    //     .into_string();

    //     todo!()
    // }
}

#[tokio::test]
async fn send_html_mail() {
    let config = crate::configuration::get_config().unwrap();
    let mailer = Mailer::new(config.smtp, config.app.origin);

    let body = html! {
        h1 {"You have been haunted by giga chad!"}
    }
    .into_string();

    let image = std::fs::read("./assets/chad.jpg").unwrap();

    let _res = mailer
        .send_mail(
            Mailbox::new(None, Address::new("adimac93", "gmail.com").unwrap()),
            "Test",
            MultiPart::mixed().multipart(
                MultiPart::related()
                    .singlepart(lettre::message::SinglePart::html(body))
                    .singlepart(
                        lettre::message::Attachment::new_inline(String::from("123"))
                            .body(image, "image/png".parse().unwrap()),
                    ),
            ),
        )
        .await;
}
