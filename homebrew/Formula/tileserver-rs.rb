# typed: false
# frozen_string_literal: true

class TileserverRs < Formula
  desc "High-performance vector tile server with native MapLibre rendering"
  homepage "https://github.com/vinayakkulkarni/tileserver-rs"
  version "2.1.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-aarch64-apple-darwin.tar.gz"
      sha256 "6276086a1f3c94a917515c9a12bb82e7cf8adcaff18ef52c037723ba2c35d9aa"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-x86_64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "TODO"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "73d3be30330a8bdf4dcd6dcc35a0a82586dc83279454f262da9857f41b598606"
    end
  end

  def install
    bin.install "tileserver-rs"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tileserver-rs --version")
  end
end
