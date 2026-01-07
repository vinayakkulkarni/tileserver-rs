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
      sha256 "62c207c9dae671df439696640acc82a9af5d7a3f35f9dc3d70d1fac1151e47b3"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-x86_64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/tileserver-rs-v#{version}/tileserver-rs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "fa010b972355502245077a6afefd8f19c5e9ab3dbfcd2e59e5c93ec11bf1ad57"
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
