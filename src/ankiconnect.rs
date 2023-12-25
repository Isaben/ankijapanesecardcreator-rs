
use serde::{Serialize, Serializer, Deserialize};
use serde::ser::SerializeStruct;

use crate::data_sources;

const ANKI_CONNECT_ADDRESS: &str = "http://localhost:8765";

struct AnkiField {
	front: String,
	back: String,
}

impl Serialize for AnkiField {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> 
	where
		S: Serializer,
	{
		let mut state = serializer.serialize_struct("AnkiField", 2)?;
		state.serialize_field("Front", &self.front)?;
		state.serialize_field("Back", &self.back)?;
		state.end()
	}
}

#[derive(Serialize)]
struct AnkiPicture {
	path: String,
	filename: String,
	fields: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AnkiOption<'a> {
	allow_duplicate: bool,
	duplicate_scope: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnkiCard<'a> {
	deck_name: String,
	model_name: &'a str,
	fields: AnkiField,
	options: AnkiOption<'a>,
	tags: Vec<String>,
	picture: Option<AnkiPicture>
}

#[derive(Serialize)]
struct PostAnkiCardParam<'a> {
	note: &'a AnkiCard<'a>,
}

#[derive(Serialize)]
struct PostAnkiCardRequest<'a> {
	action: &'a str,
	version: u8,
	params: PostAnkiCardParam<'a>,
}

#[derive(Deserialize, Debug)]
pub struct PostAnkiCardResponse {
	result: u64,
}

#[derive(Deserialize, Debug)]
pub struct GetDeckResponse {
	result: Vec<String>,
}

#[derive(Serialize, Debug)]
struct GetDeckRequest<'a> {
	action: &'a str,
	version: u8,
}


pub struct AnkiClient {
	request_client: reqwest::blocking::Client,
}

impl AnkiClient {
	pub fn new() -> AnkiClient {
		AnkiClient { request_client: reqwest::blocking::Client::new() }
	}

	pub fn get_decks(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
		let request_data = GetDeckRequest {
			action: "deckNames",
			version: 6,
		};

		let res = self.request_client.post(ANKI_CONNECT_ADDRESS).json(&request_data).send()?.json::<GetDeckResponse>()?;

		Ok(res.result)
	}
	
	pub fn add_card_to_deck(&self, card: &AnkiCard) -> Result<u64, Box<dyn std::error::Error>> {
		let anki_card_param = PostAnkiCardParam {
			note: &card
		};
		let request_data = PostAnkiCardRequest {
			action: "addNote",
			version: 6,
			params: anki_card_param
		};
		
		let res = self.request_client.post(ANKI_CONNECT_ADDRESS).json(&request_data).send()?.json::<PostAnkiCardResponse>()?;
		
		Ok(res.result)
	}
}


pub fn kanji_requests(kanjiapi_client: &data_sources::kanjiapi::KanjiAPIClient, word: String)
-> Vec<impl futures::Future<Output = Result<data_sources::kanjiapi::KanjiApiResponse, Box<dyn std::error::Error>>> + '_> {
	let mut reqs = vec![];
	for _kanji in word.chars() {
		reqs.push(kanjiapi_client.get_kanji_data(_kanji));
	}
	reqs
}

pub fn create_card<'a>(
	tokio_rt: &tokio::runtime::Runtime,
	jisho_client: &data_sources::jisho::JishoClient,
	kanjiapi_client: &data_sources::kanjiapi::KanjiAPIClient,
	current_dir: &str,
	word: &str,
	sentence: &str,
	picture: &Option<image::RgbaImage>,
	deck_name: &str
) -> Result<AnkiCard<'a>, String> {

	let (jisho_response, kanjiapi_responses) = tokio_rt.block_on(async {
		let clean_word = kanjiapi_client.remove_all_but_kanji(word.clone());

		futures::join!(jisho_client.get_jisho_data(word), futures::future::join_all(kanji_requests(&kanjiapi_client, clean_word)))
	});

	if let Err(jisho_error) = jisho_response {
		return Err(format!("Error when calling jisho: {:?}", jisho_error));
	}

	let jisho_data = jisho_response.unwrap();
	
	if jisho_data.data.len() == 0 {
		return Err(format!("No data found for the requested input. Try again with something else"));
	}

	
	let mut fields = AnkiField {
		front: format!("{}<br><br>{}<br><br>", word, sentence),
		back: String::from("")
	};

	let jisho_info = &jisho_data.data[0];
	
	// Add Jisho japanese readings
	for info in jisho_info.japanese.iter() {
		let mut reading = "".to_string();
		let mut word = "".to_string();
		
		if let Some(_reading) = info.reading.clone() {
			reading = _reading;
		}

		if let Some(_word) = info.word.clone() {
			word = _word;
		}
		
		fields.back = format!("{}{}【{}】 ", fields.back, word, reading);
	}

	fields.back += "<br><br>";
	
	// Add Jisho meanings to the back
	for sense in jisho_info.senses.iter() {
		let sep = ", ";
		let speech = sense.parts_of_speech.join(sep);
		let meaning = sense.english_definitions.join(sep);
		let mut info = "".to_string();
		
		if sense.info.len() > 0 {
			info = sense.info.join(sep);
		}
		
		if !fields.back.contains(&speech) {
			fields.back = format!("{}{}<br>", fields.back, speech);
		}
		
		fields.back = format!("{}• {} {}<br>", fields.back, meaning, info);
	}
	
	fields.back += "<br><br>";
	
	for kanjiapi_response in kanjiapi_responses.iter() {

		match kanjiapi_response {
			Err(kanjiapi_error) => {
				return Err(format!("Error when calling kanjiapi: {:?}", kanjiapi_error));
			},
			Ok(kanjiapi_data) => {
				fields.back = format!("{}{}<br>", fields.back, kanjiapi_data.kanji);
				fields.back = format!("{}{}<br>", fields.back, kanjiapi_data.meanings.join(", "));
				fields.back = format!("{}Kun: {}<br>", fields.back, kanjiapi_data.kun_readings.join(", "));
				fields.back = format!("{}On: {}<br>", fields.back, kanjiapi_data.on_readings.join(", "));
				fields.back = format!("{}Name: {}<br>", fields.back, kanjiapi_data.name_readings.join(", "));
		
				fields.back += "<br>";
			}
		}
	}

	let mut created_card = AnkiCard {
		deck_name: deck_name.to_string(),
		model_name: "Basic",
		options: AnkiOption {
			allow_duplicate: false,
			duplicate_scope: "deck"
		},
		tags: vec!["From Rust".to_string()],
		fields,
		picture: None
	};
	
	if let Some(_picture) = picture.clone() {
		let save_res = _picture.save("pic.jpg");

		if let Err(_error) = save_res {
			return Err(format!("Error happened when saving picture: {:?}\n", _error));
		}

		created_card.picture = Some(AnkiPicture {
			path: format!("{}\\pic.jpg", current_dir),
			fields: vec!["Front".to_string()],
			filename: "pic.jpg".to_string()
		});
	}

	Ok(created_card)
}
