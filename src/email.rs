use crate::ApiError;
use lettre::{
    Address, Message, SmtpTransport, Transport,
    message::{Mailbox, MessageBuilder},
    transport::smtp::authentication::Credentials,
};
use std::env;
use uuid::Uuid;

pub enum EmailType {
    EmailVerify([u8; 64], Uuid),
}

// ai generated function
pub fn check_email(email: &str) -> bool {
    // Must contain exactly one '@'
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }

    let local = parts[0];
    let domain = parts[1];

    // Local and domain parts must not be empty
    if local.is_empty() || domain.is_empty() {
        return false;
    }

    // Domain must contain at least one dot
    let domain_parts: Vec<&str> = domain.split('.').collect();
    if domain_parts.len() < 2 {
        return false;
    }

    // No empty sections in the domain (e.g. "example..com")
    if domain_parts.iter().any(|part| part.is_empty()) {
        return false;
    }

    // No spaces allowed
    if email.contains(' ') {
        return false;
    }

    true
}

pub fn send_email(email: &str, username: String, email_type: EmailType) -> Result<(), ApiError> {
    let email_first_part = MessageBuilder::new()
        .to(Mailbox {
            name: None, //Some(username.clone()),
            email: email.parse()?,
        })
        .from(Mailbox {
            name: Some("ButterflyVR".to_owned()),
            email: Address::new("support", "butterflyvr.net")?,
        });

    let email: Message;

    match email_type {
        EmailType::EmailVerify(token, user_id) => {
            email = email_first_part
                .subject("Verify your email for ButterflyVR")
                .body(format!("Dear {},

                Thank you for registering with ButterflyVR. To complete your account setup, please verify your email address by clicking the link below:

                {}

                If you did not create an account with ButterflyVR, please disregard this email.

                This link expires in 15 minutes.

                Best regards,
                The ButterflyVR Team
", username, format!("https://butterflyvr.net/api/v0/user/{}/verify/{}", user_id, hex::encode(token))))?;
        }
    }

    let mailer = SmtpTransport::starttls_relay("smtp.protonmail.ch")
        .unwrap()
        .credentials(Credentials::new(
            env::var("PROTONMAIL_EMAIL").expect("PROTONMAIL_EMAIL must be set"),
            env::var("PROTONMAIL_TOKEN").expect("PROTONMAIL_TOKEN must be set"),
        ))
        .build();
    mailer.send(&email)?;
    Ok(())
}
