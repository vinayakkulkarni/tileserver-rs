# typed: false
# frozen_string_literal: true

class TileserverRs < Formula
  desc "High-performance vector tile server with native MapLibre rendering"
  homepage "https://github.com/vinayakkulkarni/tileserver-rs"
  version "1.0.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-aarch64-apple-darwin.tar.gz"
      sha256 "8a519570ea02f6362bc667978d876d3e3bd604751640f50aaff6f1562769db63"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-x86_64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "9c62f824547cffe0db0c0caebc5636dc78e50c7035b6c87078950dbfc9276f0b"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "d1d384a8976bd74fb19d8521d6b8f79d0b40a214ad1c9c5df6327c8eaece9df2"
    end
  end

  def install
    bin.install "tileserver-rs"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tileserver-rs --version")
  end
end
