class Agnix < Formula
  desc "Lint AI agent configuration files"
  homepage "https://github.com/avifenesh/agnix"
  url "https://github.com/avifenesh/agnix/archive/refs/tags/v0.3.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/avifenesh/agnix.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/agnix-cli")
  end

  test do
    # Create a test SKILL.md with an error
    (testpath/"SKILL.md").write <<~EOS
      ---
      name: Test-Skill
      ---
      Test body
    EOS

    output = shell_output("#{bin}/agnix #{testpath}", 1)
    assert_match "Invalid name", output
  end
end
