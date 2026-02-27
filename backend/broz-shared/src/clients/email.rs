use reqwest::Client;
use serde::Serialize;

#[derive(Clone)]
pub struct EmailClient {
    client: Client,
    api_key: String,
    from_email: String,
    from_name: String,
}

#[derive(Debug, Serialize)]
struct ResendRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    html: String,
}

impl EmailClient {
    pub fn new(api_key: &str, from_email: &str, from_name: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            from_email: from_email.to_string(),
            from_name: from_name.to_string(),
        }
    }

    pub async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html: &str,
    ) -> Result<(), String> {
        let request = ResendRequest {
            from: format!("{} <{}>", self.from_name, self.from_email),
            to: vec![to.to_string()],
            subject: subject.to_string(),
            html: html.to_string(),
        };

        let response = self.client
            .post("https://api.resend.com/emails")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("email send failed: {e}"))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(format!("email API error: {body}"));
        }

        tracing::debug!(to = %to, subject = %subject, "email sent");
        Ok(())
    }

    pub async fn send_verification_code(&self, to: &str, code: &str) -> Result<(), String> {
        let html = format!(
            r#"<div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto;">
            <h2 style="color: #7c3aed;">BROZ - Email Verification</h2>
            <p>Your verification code is:</p>
            <div style="background: #1a1a2e; color: #7c3aed; font-size: 32px; font-weight: bold; text-align: center; padding: 20px; border-radius: 8px; letter-spacing: 8px;">{code}</div>
            <p style="color: #666; margin-top: 20px;">This code expires in 15 minutes.</p>
            </div>"#
        );

        self.send_email(to, "BROZ - Verify your email", &html).await
    }

    pub async fn send_password_reset_code(&self, to: &str, code: &str) -> Result<(), String> {
        let html = format!(
            r#"<div style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto;">
            <h2 style="color: #7c3aed;">BROZ - Password Reset</h2>
            <p>Your password reset code is:</p>
            <div style="background: #1a1a2e; color: #7c3aed; font-size: 32px; font-weight: bold; text-align: center; padding: 20px; border-radius: 8px; letter-spacing: 8px;">{code}</div>
            <p style="color: #666; margin-top: 20px;">This code expires in 15 minutes. If you did not request this, please ignore this email.</p>
            </div>"#
        );

        self.send_email(to, "BROZ - Reset your password", &html).await
    }
}
