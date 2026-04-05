# Kekulen API
ML-KEM-768 + HKDF (sha256) + ChaCha20-Poly1305 + I2P SAM

example of usage: https://github.com/wipedlifepotato/Kekulen-I2P-Messanger/pull/3#issuecomment-4187115098

swagger-api documentation: /swagger-ui
```
Usage: kekulen [OPTIONS]

Options:
  -c, --config <CONFIG>      [default: config.yaml]
  -d, --dat-file <DAT_FILE>  [default: my.dat]
  -p, --port <PORT>          [default: 8080]
      --password <PASSWORD>  [default: password]
  -r, --run-tui              tui instead api
  -h, --help                 Print help
```

# Android

```
rustup target add aarch64-linux-android
rustup target add armv7-linux-androideabi
rustup target add x86_64-linux-android

cargo install cargo-ndk

ANDROID_NDK_HOME=~/android-ndk-r26b cargo ndk -t aarch64-linux-android build --release
after in target will be .so library, add to your project. you can look to JNI in src/jni.rs
like this will help run on 8384 port on localhost api:
companion object {
    init {
        System.loadLibrary("kekulen")
    }
}

external fun initProtocol(datFile: String, pass: String)
external fun sendMessage(pubKey: String, message: String): String


```

# PC
1. Download rustup https://rustup.rs/
2. load .env though source
3. cargo build --release

## webui
near your executable file create directory "server", and put there index.html, that you can get in repository, after on webui port will be webui. there is not will be xss, but if you paranoid exists tui version
