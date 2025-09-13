# Murasame-chan Desktop Pet

Rust implementation of [LemonQu-GIT/MurasamePet](https://github.com/LemonQu-GIT/MurasamePet/)

## Resources

- The model editor: <https://gist.github.com/cubewhy/8791f6fbc889c15cce172dccf3977489>
- Freemote: <https://github.com/UlyssesWu/FreeMote>
- xp3-tools: <https://github.com/cubewhy/xp3-tools>
- moegirl wiki: <https://mzh.moegirl.org.cn/丛雨>

## Features

- No GPU needed - just click to run
- No hard-encoding, you're allowed to customize everything

## Usage

> Security Warning: Do not expose services on the public network, use 127.0.0.1
> if possible

- Generate an API key at [Google AI Studio](https://aistudio.google.com)
- Install [MiniConda](https://www.anaconda.com/docs/getting-started/miniconda/install#quickstart-install-instructions)
- Download and install [GPT-SoVits](https://github.com/RVC-Boss/GPT-SoVITS)
- Download the [models](https://huggingface.co/cubewhy/Murasame-chan-GPT-SoVits/)
- (Optional) Install CUDA if you are using a Nvidia GPU
- Put models into `GPT_weights_v2Pro/` and `SoVITS_weights_v2Pro/`
- Build everything by running

```shell
cargo build -r
```

- Modify the variables in `.env`
- Run the GPT-SoVITS API

```shell
# Do not copy/paste directly, change the paramaters with your own value
conda activate GPT-SoVITS
cd path/to/gpt-sovits
python api_v2.py
```

- Run the TTS servlet

```shell
./tts
```

- Write a service for pushing comments
- Run the vtuber

```shell
./vtuber
```

### Want a human-friendly Logging?

- Install bunyan-rs by running

```shell
cargo install bunyan
```

- Than run your servlet with

```shell
<servlet> | bunyan
```

## License

This work is licensed under GPL-3.0

You're allowed to use, share and modify
