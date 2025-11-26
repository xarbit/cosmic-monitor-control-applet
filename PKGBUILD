# Maintainer: Jason Scurtu <github@mail.scurtu.me>

pkgname=cosmic-monitor-control-applet
pkgver=0.2.1
pkgrel=1
pkgdesc='Control external monitors (DDC/CI and Apple displays) from COSMIC Desktop'
arch=('x86_64' 'aarch64')
url='https://github.com/xarbit/cosmic-monitor-control-applet'
license=('GPL-3.0-only')
depends=('i2c-tools' 'hidapi')
makedepends=('rust' 'cargo')
source=("$pkgname-$pkgver.tar.gz::https://github.com/xarbit/$pkgname/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname-$pkgver"
    cargo fetch --locked --target "$CARCH-unknown-linux-gnu"
}

build() {
    cd "$pkgname-$pkgver"
    export RUSTUP_TOOLCHAIN=stable
    export CARGO_TARGET_DIR=target
    cargo build --frozen --release --all-features
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --frozen --all-features
}

package() {
    cd "$pkgname-$pkgver"
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
}
