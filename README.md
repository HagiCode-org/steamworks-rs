# steamworks (HagiCode fork)
![License](https://img.shields.io/crates/l/steamworks.svg)

Rust bindings to the [Steamworks SDK](https://partner.steamgames.com/doc/sdk), maintained as a personal fork of [Noxime/steamworks-rs](https://github.com/Noxime/steamworks-rs).

Introduction and guides for the Node.js side of this work: **https://steamworks.hagicode.com/**

## About this fork

This is a fork of the upstream [steamworks-rs](https://github.com/Noxime/steamworks-rs). It exists mainly as the Rust layer of [HagiCode-org/steamworks.js](https://github.com/HagiCode-org/steamworks.js), which is itself a fork of [ceifa/steamworks.js](https://github.com/ceifa/steamworks.js) and vendors this repository as a git submodule. It is shaped for my own development work and for working with coding agents (see [AGENTS.md](./AGENTS.md)), not as a general replacement for upstream.

Additions in this fork so far, all around Steam Workshop and callbacks:

* `UpdateHandle::language`, wrapping `SteamAPI_ISteamUGC_SetItemUpdateLanguage`, to set a per-language title and description.
* Additional Workshop previews: `num_additional_previews` and `get_additional_preview` on queries, and `UpdateHandle::add_preview_file` and `remove_preview` on updates.
* Workshop item dependencies: `UGC::add_dependency` and `UGC::remove_dependency`.
* `serde` support for callback payloads, behind the `serde` feature.

Upstream structure is kept where possible, so changes can be contributed back. There is no promise of staying in step with upstream, and this fork is consumed through git or as a submodule, not from crates.io. If you want the general-purpose crate, use upstream. All credit for the original work goes to its authors. Both license files are kept as they were.

## Usage
The `steamworks` crate on crates.io is upstream. To use this fork, depend on it through git:

```toml
[dependencies]
steamworks = { git = "https://github.com/HagiCode-org/steamworks-rs" }
```

Pin a `rev` for reproducible builds.

| Crate  | SDK   | MSRV   |
| ------ | ----- | ------ |
| git (this fork) | 1.62  | 1.80.0 |
| 0.12.x (upstream, crates.io) | 1.62  | 1.80.0 |
| 0.11.0 | 1.58a | 1.71.1 |
| 0.10.0 | 1.54  | 1.56.1 |
| 0.9.0  | 1.53a | 1.56.1 |

## Example
You can find more examples in [examples](examples/).
```rust
use steamworks::AppId;
use steamworks::Client;
use steamworks::FriendFlags;
use steamworks::PersonaStateChange;

fn main() {
    let client = Client::init().unwrap();

    let _cb = client.register_callback(|p: PersonaStateChange| {
        println!("Got callback: {:?}", p);
    });

    let utils = client.utils();
    println!("Utils:");
    println!("AppId: {:?}", utils.app_id());
    println!("UI Language: {}", utils.ui_language());

    let apps = client.apps();
    println!("Apps");
    println!("IsInstalled(480): {}", apps.is_app_installed(AppId(480)));
    println!("InstallDir(480): {}", apps.app_install_dir(AppId(480)));
    println!("BuildId: {}", apps.app_build_id());
    println!("AppOwner: {:?}", apps.app_owner());
    println!("Langs: {:?}", apps.available_game_languages());
    println!("Lang: {}", apps.current_game_language());
    println!("Beta: {:?}", apps.current_beta_name());

    let friends = client.friends();
    println!("Friends");
    let list = friends.get_friends(FriendFlags::IMMEDIATE);
    println!("{:?}", list);
    for f in &list {
        println!("Friend: {:?} - {}({:?})", f.id(), f.name(), f.state());
        friends.request_user_information(f.id(), true);
    }

    for _ in 0..50 {
        client.run_callbacks();
        ::std::thread::sleep(::std::time::Duration::from_millis(100));
    }
}
```

## Features
- `serde`: serialization and deserialization of some types, including callback payloads, with `serde`.
- `image`: access image data like icons with the `image` crate.
- `raw-bindings`: re-exports the raw FFI crate as `steamworks::sys`.

Workshop item dependencies: `UGC::add_dependency` and `UGC::remove_dependency` mark a published item as requiring another. They need no feature flag, and report the Steam result code (`UGCDependencyResult::result`, `1` is OK) rather than only success or failure.

## License
Dual-licensed under [Apache](./LICENSE-APACHE) and
[MIT](./LICENSE-MIT), as upstream, except for the files in [`steamworks-sys/lib/steam/`]

## Help, I can't run my game!
If you are seeing errors like `STATUS_DLL_NOT_FOUND`, `Image not found` etc. You are likely missing the Steamworks SDK Redistributable files. Steamworks-rs loads the SDK dynamically, so the libraries need to exist somewhere the operating system can find them. This is likely next to your game binary (.exe on windows). You can find the required files in the SDK release ZIP, under `lib\steam\redistributable_bin`. See #63 for further details
