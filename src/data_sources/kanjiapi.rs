use serde::Deserialize;
use regex::Regex;

const KANJI_API_ADDRESS: &str = "https://kanjiapi.dev/v1";

#[derive(Deserialize, Debug)]
pub struct KanjiApiResponse {
	pub kanji: String,
	pub meanings: Vec<String>,
	pub kun_readings: Vec<String>,
	pub on_readings: Vec<String>,
	pub name_readings: Vec<String>,
}

pub struct KanjiAPIClient {
	request_client: reqwest::Client,
	remove_all_but_kanji: Regex,
}

impl KanjiAPIClient {
	pub fn new() -> KanjiAPIClient {
		KanjiAPIClient {
			request_client: reqwest::Client::new(),
			remove_all_but_kanji: Regex::new(r"[^\p{Han}]+|々").unwrap(),
		}
	}

	pub async fn get_kanji_data(&self, kanji: char) -> Result<KanjiApiResponse, Box<dyn std::error::Error>> {
		let request_url = format!("{}/kanji/{}", KANJI_API_ADDRESS, kanji);
		
		Ok(self.request_client.get(request_url)
		.send().await?
		.json::<KanjiApiResponse>().await?)
	}
	
	pub fn remove_all_but_kanji(&self, word: &str) -> String {
		self.remove_all_but_kanji.replace_all(word, "").to_string()
	}
}