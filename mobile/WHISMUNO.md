# WhisMuno Android (work in progress)

This directory is based on [BitChord](https://github.com/kushagrasinghx/BitChord),
snapshot `21356206de5594ca0bdaf4680ca2db68a2287729`, licensed under GPL-3.0.
The upstream copyright notices and [license](LICENSE) remain in place.

The phone application ID and default label have been changed to WhisMuno, and
the upstream app update check is disabled: upstream APKs cannot update this fork.
This is an initial Android adaptation, not a finished rebrand. It still contains
upstream branding, services and optional integrations. Review those before release.

Upstream requires Android 8 (API 26). Android 7 support has not been implemented
or tested. This source tree does not provide an iOS application.

The Android workflow builds a debug-signed APK for testing, not a production-signed
release. To build locally, install JDK 17, Android SDK 37, NDK and CMake, then run
`./gradlew :app:assembleProdDebug -Pbitchord.bundleWear=false` from this directory.
