Same as [this](https://github.com/Isaben/ankijapanesecardcreator) but using Rust with egui because why not. The only difference is the feature for adding pictures to your Anki cards, something missing from the Golang version.

## TODO

- Unblocking requests (it's already using with tokio but there is a `block_on` on the UI thread which isn't great);
- Add a spinner;
- Maybe reduce the picture size on the UI, the memory usage is not optimal.