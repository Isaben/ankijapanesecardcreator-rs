
use serde::Deserialize;

pub const JISHO_ADDRESS: &str = "https://jisho.org/api/v1";

#[derive(Deserialize, Debug)]
pub struct JishoJapanese {
	pub word: Option<String>,
	pub reading: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct JishoSense {
	pub english_definitions: Vec<String>,
	pub parts_of_speech: Vec<String>,
	pub info: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct JishoData {
	pub senses: Vec<JishoSense>,
	pub japanese: Vec<JishoJapanese>,
}

#[derive(Deserialize, Debug)]
pub struct JishoAPIResponse {
	pub data: Vec<JishoData>,
}

pub struct JishoClient {
	pub request_client: reqwest::Client,
}

impl JishoClient {
	pub fn new() -> JishoClient {
		JishoClient { request_client: reqwest::Client::new() }
	}

	pub async fn get_jisho_data(&self, word: &str) -> Result<JishoAPIResponse, Box<dyn std::error::Error>> {
		let request_url = format!("{}/search/words?keyword={}", JISHO_ADDRESS, word);

		Ok(self.request_client.get(request_url)
		.send().await?
		.json::<JishoAPIResponse>().await?)
	}
}
