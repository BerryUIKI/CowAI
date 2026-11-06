const RELEASES_API = "https://api.github.com/repos/BerryUIKI/CowBot/releases";
const RELEASES_PAGE = "https://github.com/BerryUIKI/CowBot/releases";

const ASSET_PATTERNS = {
  windows: { exe: /x64-setup\.exe$|x86_64.*\.exe$/i, msi: /\.msi$/i },
  macos: { dmg: /\.dmg$/i },
  linux: { appimage: /\.AppImage$/i, deb: /_amd64\.deb$|\.deb$/i },
};

const PLATFORM_LABELS = {
  windows: "Windows",
  macos: "macOS",
  linux: "Linux",
};

function detectPlatform() {
  const ua = navigator.userAgent;
  const platform = navigator.userAgentData?.platform || navigator.platform || "";
  if (/Mac/i.test(platform)) return "macos";
  if (/Win/i.test(platform)) return "windows";
  if (/Linux|X11/i.test(platform) && !/Android/i.test(ua)) return "linux";
  return null;
}

function findAsset(assets, patterns) {
  for (const pattern of patterns) {
    const match = assets.find((asset) => pattern.test(asset.name));
    if (match) return match;
  }
  return null;
}

function updateDownloadLinks(release) {
  for (const [os, patterns] of Object.entries(ASSET_PATTERNS)) {
    for (const [key, pattern] of Object.entries(patterns)) {
      const asset = findAsset(release.assets, [pattern]);
      if (!asset) continue;
      document
        .querySelectorAll(`[data-os-link="${os}"][data-asset="${key}"]`)
        .forEach((link) => {
          link.href = asset.browser_download_url;
        });
    }
  }

  document.querySelectorAll("[data-release-version]").forEach((node) => {
    node.textContent = `${release.tag_name} · ${new Date(
      release.published_at
    ).toLocaleDateString(undefined, { year: "numeric", month: "short" })}`;
  });

  const platform = detectPlatform();
  const heroLink = document.getElementById("hero-download");
  if (heroLink && platform) {
    const asset = findAsset(
      release.assets,
      Object.values(ASSET_PATTERNS[platform])
    );
    if (asset) heroLink.href = asset.browser_download_url;
  }
}

function setHeroLabel(platform) {
  const heroLink = document.getElementById("hero-download");
  if (!heroLink || !platform) return;
  heroLink.textContent = `Download for ${PLATFORM_LABELS[platform]}`;
}

async function loadLatestRelease() {
  try {
    const response = await fetch(`${RELEASES_API}/latest`, {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) throw new Error(`GitHub returned ${response.status}`);
    const release = await response.json();
    if (Array.isArray(release.assets)) updateDownloadLinks(release);
  } catch (error) {
    // Keep the releases-page fallbacks already in the markup.
    console.info("Using release page links:", error.message);
  }
}

function wireApproval() {
  const approval = document.getElementById("approval");
  const approve = document.getElementById("approve");
  const reject = document.getElementById("reject");
  const result = document.getElementById("approval-result");
  const reviewer = document.getElementById("reviewer-msg");
  if (!approval || !approve || !reject || !result || !reviewer) return;

  let decided = false;

  function decide(kind) {
    if (decided) return;
    decided = true;
    result.hidden = false;
    result.classList.toggle("is-rejected", kind === "rejected");
    if (kind === "approved") {
      result.textContent = "Approved — running cargo test --workspace";
      reviewer.classList.remove("is-hidden");
      reviewer.classList.add("is-new");
      approval.querySelector(".approval-actions").hidden = true;
    } else {
      result.textContent = "Rejected — the coder will revise the plan";
      approval.querySelector(".approval-actions").hidden = true;
      reviewer.classList.add("is-hidden");
    }
  }

  approve.addEventListener("click", () => decide("approved"));
  reject.addEventListener("click", () => decide("rejected"));
}

function wireCopy() {
  document.querySelectorAll("[data-copy]").forEach((button) => {
    button.addEventListener("click", async () => {
      const source = document.getElementById(button.dataset.copy);
      if (!source) return;
      try {
        await navigator.clipboard.writeText(source.textContent);
        button.textContent = "Copied";
        setTimeout(() => {
          button.textContent = "Copy";
        }, 1600);
      } catch {
        button.textContent = "Press Ctrl+C";
      }
    });
  });
}

const platform = detectPlatform();
setHeroLabel(platform);
wireApproval();
wireCopy();
loadLatestRelease();
