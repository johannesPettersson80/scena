# Demo Sample Sources

This manifest records third-party and source-derived assets used by the public
demo. Final demo assets must be reproducible from documented sources and must
not depend on local-only generated files.

## Environment HDRI

### `environment/white_studio_03_1k.hdr`

- Source: Poly Haven `white_studio_03`
- Source page: `https://polyhaven.com/a/white_studio_03`
- Direct download: `https://dl.polyhaven.org/file/ph-assets/HDRIs/hdr/1k/white_studio_03_1k.hdr`
- License: CC0
- Download date: 2026-05-17
- Selected resolution: 1K Radiance HDR
- Original file name: `white_studio_03_1k.hdr`
- Demo path: `demo/samples/environment/white_studio_03_1k.hdr`
- SHA-256: `ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b`
- Transfer/disk size: 1,390,531 bytes
- Measured mean RGB via ImageMagick resize-to-1x1: `0.451606, 0.448569, 0.430823`
- Demo role: selected neutral studio HDRI for connector, Khronos PBR, and
  Khronos vehicle pages.

## Connector Materials

### `materials/ambientcg/Metal030`

- Source: ambientCG `Metal030`
- Source page: `https://ambientcg.com/a/Metal030`
- Direct download used: `https://ambientcg.com/get?file=Metal030_1K-JPG.zip`
- License: CC0
- Download date: 2026-05-17
- Selected resolution: 1K JPG
- Original archive SHA-256:
  `6fc0f022e80a12e3807bee3a64362c4a9fc6e1ad1995d84701ace83ed1f4acd1`
- Demo role: first controlled connector material replacement for the repeated
  `brushed steel` material in drive, load, and assembled connector GLBs.
- Stored source files:
  - `source/Metal030_1K-JPG_Color.jpg` SHA-256
    `f909ca1206b256d73d84023393ae5b45e308fb60fb904afb54942ddd1a9f1f46`
  - `source/Metal030_1K-JPG_NormalGL.jpg` SHA-256
    `5b805d1b20afd0bf0d0d8e380f738caa8b42bcfbf25b739c457226cd7e82576f`
  - `source/Metal030_1K-JPG_Roughness.jpg` SHA-256
    `42c8c55e4e7ea87ef79b3afe245b625c39827d262aba8fdf97dd33e976c044b3`
  - `source/Metal030_1K-JPG_Metalness.jpg` SHA-256
    `843102638fa0a5bf897888868877efab8ccdc4cb84a48e57bdb52bfb30f5e714`
  - `source/Metal030_1K-JPG_Displacement.jpg` SHA-256
    `e824ccdc24e2a34c585c65bf588ca1fb7f38a7ccf9dbea932ba2235bc88480b6`
- Derived demo texture:
  - `Metal030_1K-JPG_OcclusionRoughnessMetallic.png` SHA-256
    `09b2b1eebf0dfe076601d853a892629ef829b6166eac361625168a300df0ef7c`
  - Derived with ImageMagick from a neutral white occlusion channel plus the
    source roughness and metalness maps:
    `magick -size 1024x1024 xc:white Metal030_1K-JPG_Roughness.jpg Metal030_1K-JPG_Metalness.jpg -combine ...`
- Application script: `scripts/apply_connector_material_textures.js` embeds
  these maps into the GLBs so the connector demo still loads as single-file GLB
  samples.
- Optimized public-demo 512px textures:
  - `demo-512/Metal030_512_Color.jpg` SHA-256
    `6cdd18fd73070320eaf71ad3675a0d7de28479c0aff444563c149d362e4e2ab9`
  - `demo-512/Metal030_512_NormalGL.jpg` SHA-256
    `d7c85632da0f228cf6d1f137fd5a74c426c6ebd3336bfde3c89d22ef4cd3c02f`
  - `demo-512/Metal030_512_OcclusionRoughnessMetallic.png` SHA-256
    `054f00c5cd471a4db7321f4b8b891e891a11ccc526648a25a8354eba27a9b4e7`

### `materials/ambientcg/Metal010`

- Source: ambientCG `Metal010`
- Source page: `https://ambientcg.com/a/Metal010`
- Direct download used: `https://ambientcg.com/get?file=Metal010_1K-JPG.zip`
- License: CC0
- Download date: 2026-05-17
- Selected resolution: 1K JPG
- Original archive SHA-256:
  `01b03853f1937aaa98870228912f7f73eb574af3b12035b7b4e1fd879a6039ea`
- Demo role: second controlled connector material replacement for the repeated
  `machined aluminium` material in drive, load, and assembled connector GLBs.
- Stored source files:
  - `source/Metal010_1K-JPG_Color.jpg` SHA-256
    `f1137f5b80564670faad3611658819325c001eceb65c52b2a3392c6e80f72998`
  - `source/Metal010_1K-JPG_NormalGL.jpg` SHA-256
    `634d074fe971056ce800e8c982cb29a9bec3f377d08b2c30f7e5efd837f751e6`
  - `source/Metal010_1K-JPG_Roughness.jpg` SHA-256
    `d37fb03651cc792aaaa0bde4ae2d4820cd7eddac468f418fff62119e487b5063`
  - `source/Metal010_1K-JPG_Metalness.jpg` SHA-256
    `7591621409d475bb0a06b3d084b58f8e90a18186c0e61a3d4e9eb418e975efc0`
  - `source/Metal010_1K-JPG_Displacement.jpg` SHA-256
    `12aaf06d2db6a3078761b06478776cc4859b6c430911f45dd90e54c1959679f0`
- Derived demo texture:
  - `Metal010_1K-JPG_OcclusionRoughnessMetallic.png` SHA-256
    `ea9be532a6b1041c8dcdad68568b209393d9bd022c9c976f5ce992bc11bbea19`
  - Derived with ImageMagick from a neutral white occlusion channel plus the
    source roughness and metalness maps:
    `magick -size 1024x1024 xc:white Metal010_1K-JPG_Roughness.jpg Metal010_1K-JPG_Metalness.jpg -combine ...`
- Application script: `scripts/apply_connector_material_textures.js` embeds
  these maps into the GLBs so the connector demo still loads as single-file GLB
  samples.
- Optimized public-demo 512px textures:
  - `demo-512/Metal010_512_Color.jpg` SHA-256
    `e11475eb84019d72dfe124736c7748f0b71fd23d40e850408628018ac1e01f79`
  - `demo-512/Metal010_512_NormalGL.jpg` SHA-256
    `c9f43ab2a097e31e28d5c8a057bd9f747d784678f14bc362cca019b723369eef`
  - `demo-512/Metal010_512_OcclusionRoughnessMetallic.png` SHA-256
    `59c42deddcd91196c4032d6f35637cf6bf993d213bd67a44bccec907cf695a0c`

### `materials/ambientcg/Rubber002`

- Source: ambientCG `Rubber002`
- Source page: `https://ambientcg.com/a/Rubber002`
- Direct download used: `https://ambientcg.com/get?file=Rubber002_1K-JPG.zip`
- License: CC0
- Download date: 2026-05-17
- Selected resolution: 1K JPG
- Original archive SHA-256:
  `d4dd369445979e7e06de0c1d2a2bd4c81f667f093a622fb472fc0a606ee935e5`
- Demo role: rubber/isolator material for connector GLBs.
- Stored source files:
  - `source/Rubber002_1K-JPG_Color.jpg` SHA-256
    `3a5ca3db72bf2c5a543e27091237ac63484180e170197caddb51bcfbeee8db37`
  - `source/Rubber002_1K-JPG_NormalGL.jpg` SHA-256
    `02709d839b1a1f402834920b8b8816920bbe08a40463aa188905a87621d6fa65`
  - `source/Rubber002_1K-JPG_Roughness.jpg` SHA-256
    `82417913bb17853c2ad8bd4c7049fe43a594be78f61b2791a0e9c9b8e3145529`
  - `source/Rubber002_1K-JPG_Displacement.jpg` SHA-256
    `4d6fd737f8f14f0ea6c352513f3ca8aeb662d110b0a7dbd4ec7b8e0001111925`
- Derived demo texture:
  - `Rubber002_1K-JPG_OcclusionRoughnessMetallic.png` SHA-256
    `7d8c7627f478e0abc6e97a765105a38a99db77c2d129e9c5f798f3a203f04e23`
  - Derived with ImageMagick from a neutral white occlusion channel, source
    roughness, and a black metalness channel.
- Application script: `scripts/apply_connector_material_textures.js` embeds
  these maps into the GLBs so the connector demo still loads as single-file GLB
  samples.
- Optimized public-demo 512px textures:
  - `demo-512/Rubber002_512_Color.jpg` SHA-256
    `1d15a210155f53abb0d7e7fccacdceaf9b37d42f59d49027c2a65e20fa895ec0`
  - `demo-512/Rubber002_512_NormalGL.jpg` SHA-256
    `fa03218d4ef7b18ed39082bfdf9bc3925838806a299c56dc433c7d0c201a8c62`
  - `demo-512/Rubber002_512_OcclusionRoughnessMetallic.png` SHA-256
    `9a61567504f1eac44610568b6d8216439d9b5f065b8f7c609a351ff546768c5b`
