# typed: false
# frozen_string_literal: true

class TileserverRs < Formula
  desc "High-performance vector tile server with native MapLibre rendering"
  homepage "https://github.com/vinayakkulkarni/tileserver-rs"
  version "2.30.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-aarch64-apple-darwin.tar.gz"
      sha256 "2b74bd1611f34b22306f41e048dd03f8c454b8f417b661758cc6f7540af7227f"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-x86_64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "0f001aa337f83936b5b963aa42eda4c2b140585d553b688956c73ddaef033683"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "7ab0954ae6ac3a8ac4aca977d0b87caf681b10215e82587555a95819f266e25d"
    end
  end

  def install
    bin.install "tileserver-rs"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tileserver-rs --version")
  end
end
