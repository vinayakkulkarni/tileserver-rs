# typed: false
# frozen_string_literal: true

class TileserverRs < Formula
  desc "High-performance vector tile server with native MapLibre rendering"
  homepage "https://github.com/vinayakkulkarni/tileserver-rs"
  version "2.5.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-aarch64-apple-darwin.tar.gz"
      sha256 "f82fb92705112da9d80f72b0e61ae30e05c0a91d4f0952271dbbb2be48afcf20"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-x86_64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e0b1d98d57c764b772e6419bbc0d4444213e2878e6e45fe7c10eb56d9596c405"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "d07356971f8689034eb00bd34505cd6a693eb0d9002958917f6c6f10f2ca872e"
    end
  end

  def install
    bin.install "tileserver-rs"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tileserver-rs --version")
  end
end
