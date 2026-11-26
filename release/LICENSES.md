# Licenses and notices

SeedFinder bundles third-party libraries and artwork. This file ships with
every release package and lists the terms under which those parts are
redistributed.

## cubiomes

World generation comes from the
[`xpple/cubiomes`](https://github.com/xpple/cubiomes) fork (upstream
`Cubitect/cubiomes`), MIT licensed. The sources live in
`src-tauri/vendor/cubiomes`.

```
MIT License

Copyright (c) 2020 Cubitect

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## loot library

The loot table evaluation in cubiomes uses the loot library, MIT licensed
(`src-tauri/vendor/cubiomes/loot/LICENSE_loot_library.h.txt`).

```
MIT License

Copyright (c) 2024 ScriptLine

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Marker sprites

The item sprites on the map (bell, crossbow, prismarine shard, eye of ender and
so on) come from the Minecraft Wiki and are Mojang property. They are bundled
locally and never fetched at runtime. SeedFinder is not affiliated with Mojang
or Microsoft.

## Other dependencies

Tauri 2 (MIT or Apache-2.0), React 19 (MIT), Vite (MIT), Serde (MIT or
Apache-2.0) and the remaining Rust and npm packages use their usual permissive
licenses. Full texts ship with the packages listed in `Cargo.lock` and
`package-lock.json`.

## Project code

No license has been chosen for the SeedFinder source itself yet.
