# typed: false
# frozen_string_literal: true

class TileserverRs < Formula
  desc "High-performance vector tile server with native MapLibre rendering"
  homepage "https://github.com/vinayakkulkarni/tileserver-rs"
  version "0.2.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-aarch64-apple-darwin.tar.gz"
      sha256 "e10dacb37203e52fdb461eb3e4ac85a67e2db40bdc71c7899a42821c644f57cb"
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
      sha256 "d4014342360c2ecee14bea99ee3e6f0ee2d57c57ebc99d7e66a715bebc17997b"
    end
  end

  def install
    bin.install "tileserver-rs"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tileserver-rs --version")
  end
end
