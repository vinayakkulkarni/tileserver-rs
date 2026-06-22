# typed: false
# frozen_string_literal: true

class TileserverRs < Formula
  desc "High-performance vector tile server with native MapLibre rendering"
  homepage "https://github.com/vinayakkulkarni/tileserver-rs"
  version "2.33.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-aarch64-apple-darwin.tar.gz"
      sha256 "d2c7d390f69fd7c9bfbb83867bc8dd2414b4fed9474eaa66c84295acbbcd0f0b"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-x86_64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "e8fcdf90efbee2ad33c23b2b83c753da68c54cb779b1a981aef1112341e525c3"
    elsif Hardware::CPU.intel?
      url "https://github.com/vinayakkulkarni/tileserver-rs/releases/download/v#{version}/tileserver-rs-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "4cc2161e9675fe9aea5eb62250b15da994cefc1e1fe6a28eba10467d8d3b84e3"
    end
  end

  def install
    bin.install "tileserver-rs"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/tileserver-rs --version")
  end
end
