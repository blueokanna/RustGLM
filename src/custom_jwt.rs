mod time_stamp;

use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64url::encode;

pub struct CustomJwt {
    secret: String,
    header: String,
    payload: String,
}

impl CustomJwt {
    pub fn new(user_id: &str, user_secret: &str) -> CustomJwt {
        let header = "{\"alg\":\"HS256\",\"sign_type\":\"SIGN\"}".to_string();
        let payload = CustomJwt::jwt_payload(user_id);
        CustomJwt {
            secret: user_secret.to_string(),
            header,
            payload,
        }
    }

    pub fn create_jwt(&self) -> String {
        let encoded_header = CustomJwt::encode_base64_url(self.header.as_bytes());
        let encoded_payload = CustomJwt::encode_base64_url(self.payload.as_bytes());
        let to_sign = format!("{}.{}", encoded_header, encoded_payload);

        let signature_bytes = self.generate_signature(&to_sign);
        let calculated_signature = CustomJwt::encode_base64_url(&signature_bytes);
        format!("{}.{}", to_sign, calculated_signature)
    }

    pub fn verify_jwt(&self, jwt: &str) -> bool {
        let jwt = jwt.trim();

        let parts: Vec<&str> = jwt.split('.').collect();
        if parts.len() != 3 {
            return false;
        }

        let encoded_header = parts[0];
        let encoded_payload = parts[1];
        let signature = parts[2];

        let to_verify = format!("{}.{}", encoded_header, encoded_payload);
        let calculated_signature_bytes = self.generate_signature(&to_verify);
        let calculated_signature = CustomJwt::encode_base64_url(&calculated_signature_bytes);

        calculated_signature == signature
    }

    fn jwt_payload(user_id: &str) -> String {
        let time_now = time_stamp::time_sync();
        let exp_time = time_now * 3;
        format!("{{\"api_key\":\"{}\",\"exp\":{},\"timestamp\":{}}}", user_id, exp_time, time_now)
    }

    fn generate_signature(&self, data: &str) -> Vec<u8> {
        let mut hmac = Hmac::<Sha256>::new_from_slice(self.secret.as_bytes())
            .expect("HMAC key error");
        hmac.update(data.as_bytes());
        hmac.finalize().into_bytes().to_vec()
    }

    fn encode_base64_url(data: &[u8]) -> String {
        encode(data)
    }
}
