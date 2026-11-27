const invoke = window.__TAURI__?.core?.invoke;

const state = {
  snapshot: null,
  speedMap: new Map(),
  isTestingSpeed: false,
  isBusy: false,
};

// DOM 引用
const els = {
  activeMirrorLabel: document.getElementById("active-mirror-label"),
  activeStatusPill: document.getElementById("active-status-pill"),
  syncRootWrap: document.getElementById("sync-root-wrap"),
  syncRootToggle: document.getElementById("sync-root-toggle"),
  syncEnvMetaToggle: document.getElementById("sync-env-meta-toggle"),
  autoCleanToggle: document.getElementById("auto-clean-toggle"),
  btnSpeedtest: document.getElementById("btn-speedtest"),
  btnCleanCache: document.getElementById("btn-clean-cache"),
  btnAddMirror: document.getElementById("btn-add-mirror"),
  btnVerifyEffective: document.getElementById("btn-verify-effective"),
  btnRefreshCondarc: document.getElementById("btn-refresh-condarc"),
  btnSnapshotFromPreview: document.getElementById("btn-snapshot-from-preview"),
  btnCreateBackup: document.getElementById("btn-create-backup"),
  mismatchBanner: document.getElementById("mismatch-banner"),
  mismatchDetail: document.getElementById("mismatch-detail"),
  mirrorGrid: document.getElementById("mirror-grid"),
  condarcPathDisplay: document.getElementById("condarc-path-display"),
  condarcContentView: document.getElementById("condarc-content-view"),
  condaExeStatus: document.getElementById("conda-exe-status"),
  rootCondarcCard: document.getElementById("root-condarc-card"),
  rootCondarcPath: document.getElementById("root-condarc-path"),
  rootCondarcBadge: document.getElementById("root-condarc-badge"),
  rootCondarcContentView: document.getElementById("root-condarc-content-view"),
  effectiveConfigCard: document.getElementById("effective-config-card"),
  effectiveConfigView: document.getElementById("effective-config-view"),
  backupList: document.getElementById("backup-list"),
  backupCountBadge: document.getElementById("backup-count-badge"),
  toastContainer: document.getElementById("toast-container"),

  mirrorModal: document.getElementById("mirror-modal"),
  mirrorModalTitle: document.getElementById("mirror-modal-title"),
  mirrorForm: document.getElementById("mirror-form"),
  fieldMirrorId: document.getElementById("field-mirror-id"),
  fieldMirrorName: document.getElementById("field-mirror-name"),
  fieldMirrorDesc: document.getElementById("field-mirror-desc"),
  fieldDefaultChannels: document.getElementById("field-default-channels"),
  fieldCustomChannels: document.getElementById("field-custom-channels"),
  fieldProbeUrl: document.getElementById("field-probe-url"),
  btnCloseMirrorModal: document.getElementById("btn-close-mirror-modal"),
  btnCancelMirrorModal: document.getElementById("btn-cancel-mirror-modal"),

  backupPreviewModal: document.getElementById("backup-preview-modal"),
  backupPreviewTitle: document.getElementById("backup-preview-title"),
  backupPreviewCode: document.getElementById("backup-preview-code"),
  btnClosePreviewModal: document.getElementById("btn-close-preview-modal"),
  btnOkPreviewModal: document.getElementById("btn-ok-preview-modal"),
};

function showToast(message, type = "success") {
  const el = document.createElement("div");
  el.className = `toast ${type}`;
  el.textContent = message;
  els.toastContainer.appendChild(el);
  setTimeout(() => {
    el.style.opacity = "0";
    el.style.transition = "opacity 180ms ease";
    setTimeout(() => el.remove(), 200);
  }, 3600);
}

function escapeHtml(str) {
  return String(str)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

function extractHost(url) {
  try {
    return new URL(url).host;
  } catch {
    return url || "-";
  }
}

function formatBytes(bytes) {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function renderLatencyBadge(mirrorId) {
  if (state.isTestingSpeed) {
    return `<span class="latency-pill">测速中...</span>`;
  }
  const res = state.speedMap.get(mirrorId);
  if (!res) {
    return `<span class="latency-pill">未测速</span>`;
  }
  if (!res.ok || res.latency_ms == null) {
    return `<span class="latency-pill slow" title="${escapeHtml(res.error || "不可达")}">${escapeHtml(res.error || "失败")}</span>`;
  }
  const ms = res.latency_ms;
  const cls = ms <= 200 ? "fast" : ms <= 600 ? "medium" : "slow";
  return `<span class="latency-pill ${cls}" title="HTTP ${res.status_code ?? ""}">${ms} ms</span>`;
}

function isRootMismatched(snap) {
  return Boolean(
    snap.condarc.root_condarc_path &&
      snap.condarc.root_active_mirror_id !== snap.condarc.active_mirror_id
  );
}

function renderAll() {
  const snap = state.snapshot;
  if (!snap) return;

  const hasRoot = Boolean(snap.condarc.root_condarc_path);
  const mismatched = isRootMismatched(snap);

  // 是否显示“同步根目录”开关
  els.syncRootWrap.classList.toggle("hidden", !hasRoot);

  // 顶部当前状态
  els.activeMirrorLabel.textContent = mismatched
    ? `当前源：${snap.condarc.active_mirror_name}（根目录不一致）`
    : `当前源：${snap.condarc.active_mirror_name}`;
  const dot = els.activeStatusPill.querySelector(".status-dot");
  if (dot) {
    dot.classList.toggle("warn", !snap.condarc.active_mirror_id || mismatched);
  }

  // 不一致警告横幅
  if (mismatched) {
    els.mismatchDetail.textContent = `用户级为「${snap.condarc.active_mirror_name}」，根目录为「${snap.condarc.root_active_mirror_name || "未知"}」。`;
    els.mismatchBanner.classList.remove("hidden");
  } else {
    els.mismatchBanner.classList.add("hidden");
  }

  // 备份角标
  els.backupCountBadge.textContent = String(snap.backups.length);

  // 渲染镜像卡片
  renderMirrors(snap, mismatched);

  // 渲染 .condarc 预览
  els.condarcPathDisplay.textContent = `用户级：${snap.condarc.path}`;
  els.condarcContentView.textContent = snap.condarc.exists
    ? snap.condarc.raw_content || "# 文件为空"
    : "# 用户级 .condarc 尚未创建，首次切换镜像源时将自动生成。";

  els.condaExeStatus.textContent = snap.conda_executable
    ? `已检测到 Conda：${snap.conda_executable}`
    : "未检测到本机 Conda 可执行文件";

  if (hasRoot) {
    els.rootCondarcPath.textContent = `根目录：${snap.condarc.root_condarc_path}`;
    els.rootCondarcBadge.textContent = `当前：${snap.condarc.root_active_mirror_name || "未知"}`;
    els.rootCondarcContentView.textContent = snap.condarc.root_condarc_content || "# 文件为空";
    els.rootCondarcCard.classList.remove("hidden");
  } else {
    els.rootCondarcCard.classList.add("hidden");
  }

  // 渲染备份列表
  renderBackups(snap);
}

function renderMirrors(snap, mismatched) {
  const activeId = snap.condarc.active_mirror_id;

  els.mirrorGrid.innerHTML = snap.mirrors
    .map((m) => {
      const isActive = activeId === m.id && !mismatched;
      const isUserOnlyActive = activeId === m.id && mismatched;
      const sampleUrl = m.custom_channels["conda-forge"] || m.default_channels[0] || m.probe_url;
      const host = extractHost(sampleUrl);
      const defCount = m.default_channels.length;
      const custKeys = Object.keys(m.custom_channels);

      return `
        <article class="mirror-card ${isActive ? "is-active" : ""}">
          <div class="mirror-card-top">
            <div class="mirror-title-wrap">
              <h3 class="mirror-name">${escapeHtml(m.name)}</h3>
              <div class="mirror-badges">
                ${isActive ? `<span class="badge badge-active">当前使用中</span>` : ""}
                ${isUserOnlyActive ? `<span class="badge">仅用户级已选</span>` : ""}
                ${m.is_builtin ? `<span class="badge">内置预设</span>` : `<span class="badge badge-custom">自定义</span>`}
              </div>
            </div>
            ${renderLatencyBadge(m.id)}
          </div>

          <p class="mirror-desc">${escapeHtml(m.description || "自定义 Conda 镜像源配置")}</p>

          <div class="mirror-meta mono">
            <div class="mirror-meta-row">
              <span>主机</span>
              <span class="mirror-meta-host" title="${escapeHtml(host)}">${escapeHtml(host)}</span>
            </div>
            <div class="mirror-meta-row">
              <span>default / custom</span>
              <span>${defCount} 项 / ${custKeys.length} 项</span>
            </div>
          </div>

          <div class="mirror-card-footer">
            <button
              class="btn ${isActive ? "btn-secondary" : "btn-primary"} btn-switch"
              type="button"
              data-action="switch"
              data-id="${escapeHtml(m.id)}"
              ${isActive || state.isBusy ? "disabled" : ""}
            >
              ${isActive ? "已启用" : isUserOnlyActive ? "同步统一到此源" : "切换到此源"}
            </button>
            ${
              !m.is_builtin
                ? `
              <div class="custom-card-actions">
                <button class="icon-btn" type="button" data-action="edit" data-id="${escapeHtml(m.id)}" aria-label="编辑自定义源" title="编辑">
                  <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                    <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" />
                    <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" />
                  </svg>
                </button>
                <button class="icon-btn danger" type="button" data-action="delete-mirror" data-id="${escapeHtml(m.id)}" aria-label="删除自定义源" title="删除">
                  <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true">
                    <polyline points="3 6 5 6 21 6" />
                    <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
                    <path d="M10 11v6M14 11v6" />
                  </svg>
                </button>
              </div>`
                : ""
            }
          </div>
        </article>
      `;
    })
    .join("");
}

function renderBackups(snap) {
  if (!snap.backups.length) {
    els.backupList.innerHTML = `
      <div class="empty-state">
        暂无备份文件。首次切换镜像源时会自动备份原始 <code>.condarc</code>，您也可以点击右上角手动创建快照。
      </div>
    `;
    return;
  }

  els.backupList.innerHTML = snap.backups
    .map(
      (b) => `
      <div class="backup-item">
        <div class="backup-info">
          <div class="backup-title-row">
            <span class="backup-title">${escapeHtml(b.label)}</span>
            ${b.is_original ? `<span class="badge badge-active">初始保护</span>` : ""}
          </div>
          <div class="backup-sub mono">
            <span>${escapeHtml(b.filename)}</span>
            <span>${escapeHtml(b.created_at)}</span>
            <span>${formatBytes(b.size_bytes)}</span>
          </div>
        </div>
        <div class="backup-actions">
          <button class="btn btn-secondary btn-sm" type="button" data-action="preview-backup" data-file="${escapeHtml(b.filename)}">预览</button>
          <button class="btn btn-primary btn-sm" type="button" data-action="restore-backup" data-file="${escapeHtml(b.filename)}">还原</button>
          ${
            !b.is_original
              ? `<button class="btn btn-danger btn-sm" type="button" data-action="delete-backup" data-file="${escapeHtml(b.filename)}">删除</button>`
              : ""
          }
        </div>
      </div>
    `
    )
    .join("");
}

async function loadSnapshot() {
  try {
    state.snapshot = await invoke("get_snapshot");
    renderAll();
  } catch (err) {
    showToast(`读取配置失败：${err}`, "error");
  }
}

async function handleSwitchMirror(mirrorId) {
  if (state.isBusy) return;
  state.isBusy = true;
  renderAll();
  try {
    const autoClean = els.autoCleanToggle.checked;
    const syncRoot = els.syncRootToggle.checked;
    const syncEnvMeta = els.syncEnvMetaToggle.checked;
    const result = await invoke("switch_mirror", { mirrorId, autoClean, syncRoot, syncEnvMeta });
    state.snapshot = result.snapshot;

    let msg = `已切换至：${result.snapshot.condarc.active_mirror_name}`;
    if (syncRoot && result.snapshot.condarc.root_condarc_path) {
      msg += "（已同步更新用户级与根目录 .condarc）";
    }
    if (result.backup_created) {
      msg += `，并自动创建初始备份`;
    }
    showToast(msg, "success");

    if (result.env_meta_sync) {
      const { updated_files, touched_envs, permission_denied_envs } = result.env_meta_sync;
      if (updated_files > 0) {
        showToast(`已同步规范化 ${updated_files} 个包索引（${touched_envs.join(", ")}）`, "info");
      }
      if (permission_denied_envs.length > 0) {
        showToast(
          `部分环境索引无写入权限（${permission_denied_envs.join(", ")}），右键「以管理员身份运行」即可自动同步`,
          "error"
        );
      }
    }

    if (result.clean_result) {
      if (result.clean_result.ok) {
        showToast("索引缓存已清理完成（conda clean -i）", "info");
      } else {
        showToast(`缓存清理未成功：${result.clean_result.stderr || "请检查 conda 环境"}`, "error");
      }
    }
  } catch (err) {
    showToast(`切换失败：${err}`, "error");
  } finally {
    state.isBusy = false;
    renderAll();
  }
}

async function handleSpeedtest() {
  if (state.isTestingSpeed) return;
  state.isTestingSpeed = true;
  els.btnSpeedtest.disabled = true;
  renderAll();
  try {
    const results = await invoke("run_speedtest");
    state.speedMap.clear();
    for (const r of results) {
      state.speedMap.set(r.mirror_id, r);
    }
    showToast("镜像源延迟测速完成", "info");
  } catch (err) {
    showToast(`测速失败：${err}`, "error");
  } finally {
    state.isTestingSpeed = false;
    els.btnSpeedtest.disabled = false;
    renderAll();
  }
}

async function handleCleanCache() {
  els.btnCleanCache.disabled = true;
  showToast("正在执行 conda clean -i ...", "info");
  try {
    const res = await invoke("clean_conda_index");
    if (res.ok) {
      showToast("Conda 索引缓存清理完成", "success");
    } else {
      showToast(`清理失败：${res.stderr || res.stdout || "未知错误"}`, "error");
    }
  } catch (err) {
    showToast(`清理失败：${err}`, "error");
  } finally {
    els.btnCleanCache.disabled = false;
  }
}

// 自定义镜像弹窗逻辑
function openMirrorModal(mirror = null) {
  els.mirrorModalTitle.textContent = mirror ? "编辑自定义镜像源" : "新增自定义镜像源";
  els.fieldMirrorId.value = mirror?.id || "";
  els.fieldMirrorName.value = mirror?.name || "";
  els.fieldMirrorDesc.value = mirror?.description || "";
  els.fieldDefaultChannels.value = (mirror?.default_channels || []).join("\n");
  els.fieldCustomChannels.value = mirror
    ? Object.entries(mirror.custom_channels)
        .map(([k, v]) => `${k}=${v}`)
        .join("\n")
    : "conda-forge=\npytorch=\nbioconda=";
  els.fieldProbeUrl.value = mirror?.probe_url || "";
  els.mirrorModal.classList.remove("hidden");
  els.fieldMirrorName.focus();
}

function closeMirrorModal() {
  els.mirrorModal.classList.add("hidden");
}

async function handleSaveCustomMirror(evt) {
  evt.preventDefault();
  const defaultChannels = els.fieldDefaultChannels.value
    .split(/\r?\n/)
    .map((s) => s.trim())
    .filter(Boolean);

  const customChannels = {};
  for (const line of els.fieldCustomChannels.value.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const idx = trimmed.indexOf("=");
    if (idx <= 0) continue;
    const key = trimmed.slice(0, idx).trim();
    const val = trimmed.slice(idx + 1).trim();
    if (key && val) {
      customChannels[key] = val;
    }
  }

  const input = {
    id: els.fieldMirrorId.value.trim() || null,
    name: els.fieldMirrorName.value.trim(),
    description: els.fieldMirrorDesc.value.trim(),
    default_channels: defaultChannels,
    custom_channels: customChannels,
    probe_url: els.fieldProbeUrl.value.trim(),
  };

  try {
    state.snapshot = await invoke("save_custom_mirror", { input });
    closeMirrorModal();
    renderAll();
    showToast(`已保存镜像源「${input.name}」`, "success");
  } catch (err) {
    showToast(`保存失败：${err}`, "error");
  }
}

// 标签页切换
document.querySelectorAll(".tab-btn").forEach((btn) => {
  btn.addEventListener("click", () => {
    const target = btn.dataset.tab;
    document.querySelectorAll(".tab-btn").forEach((b) => {
      const active = b.dataset.tab === target;
      b.classList.toggle("active", active);
      b.setAttribute("aria-selected", String(active));
    });
    document.querySelectorAll(".tab-panel").forEach((panel) => {
      panel.classList.toggle("active", panel.id === `panel-${target}`);
    });
  });
});

// 顶部按钮事件
els.btnSpeedtest.addEventListener("click", handleSpeedtest);
els.btnCleanCache.addEventListener("click", handleCleanCache);
els.btnAddMirror.addEventListener("click", () => openMirrorModal(null));
els.btnVerifyEffective.addEventListener("click", async () => {
  els.btnVerifyEffective.disabled = true;
  try {
    const output = await invoke("get_effective_conda_config");
    els.effectiveConfigView.textContent = output || "# 无输出";
    els.effectiveConfigCard.classList.remove("hidden");
    showToast("已获取 Conda 实际生效配置", "info");
  } catch (err) {
    showToast(`获取生效配置失败：${err}`, "error");
  } finally {
    els.btnVerifyEffective.disabled = false;
  }
});
els.btnRefreshCondarc.addEventListener("click", async () => {
  await loadSnapshot();
  showToast("已重新读取 .condarc", "info");
});

async function handleCreateBackup() {
  try {
    state.snapshot = await invoke("create_backup");
    renderAll();
    showToast("已创建当前 .condarc 快照备份", "success");
  } catch (err) {
    showToast(`创建备份失败：${err}`, "error");
  }
}

els.btnSnapshotFromPreview.addEventListener("click", handleCreateBackup);
els.btnCreateBackup.addEventListener("click", handleCreateBackup);

// 镜像网格事件委托
els.mirrorGrid.addEventListener("click", async (e) => {
  const btn = e.target.closest("button[data-action]");
  if (!btn) return;
  const action = btn.dataset.action;
  const id = btn.dataset.id;

  if (action === "switch") {
    await handleSwitchMirror(id);
  } else if (action === "edit") {
    const mirror = state.snapshot?.mirrors.find((m) => m.id === id);
    if (mirror) openMirrorModal(mirror);
  } else if (action === "delete-mirror") {
    if (!confirm("确定要删除这个自定义镜像源吗？")) return;
    try {
      state.snapshot = await invoke("remove_custom_mirror", { mirrorId: id });
      renderAll();
      showToast("已删除自定义镜像源", "info");
    } catch (err) {
      showToast(`删除失败：${err}`, "error");
    }
  }
});

// 备份列表事件委托
els.backupList.addEventListener("click", async (e) => {
  const btn = e.target.closest("button[data-action]");
  if (!btn) return;
  const action = btn.dataset.action;
  const filename = btn.dataset.file;

  if (action === "preview-backup") {
    try {
      const content = await invoke("preview_backup", { filename });
      els.backupPreviewTitle.textContent = `备份预览 — ${filename}`;
      els.backupPreviewCode.textContent = content || "# 空文件";
      els.backupPreviewModal.classList.remove("hidden");
    } catch (err) {
      showToast(`读取备份失败：${err}`, "error");
    }
  } else if (action === "restore-backup") {
    if (!confirm(`确定要将 .condarc 还原为备份 ${filename} 吗？`)) return;
    try {
      state.snapshot = await invoke("restore_backup", { filename });
      renderAll();
      showToast(`已成功还原备份 ${filename}`, "success");
    } catch (err) {
      showToast(`还原失败：${err}`, "error");
    }
  } else if (action === "delete-backup") {
    if (!confirm(`确定要删除备份 ${filename} 吗？`)) return;
    try {
      state.snapshot = await invoke("remove_backup", { filename });
      renderAll();
      showToast("备份已删除", "info");
    } catch (err) {
      showToast(`删除失败：${err}`, "error");
    }
  }
});

// 弹窗关闭与表单提交
els.mirrorForm.addEventListener("submit", handleSaveCustomMirror);
els.btnCloseMirrorModal.addEventListener("click", closeMirrorModal);
els.btnCancelMirrorModal.addEventListener("click", closeMirrorModal);
els.btnClosePreviewModal.addEventListener("click", () => els.backupPreviewModal.classList.add("hidden"));
els.btnOkPreviewModal.addEventListener("click", () => els.backupPreviewModal.classList.add("hidden"));

window.addEventListener("keydown", (e) => {
  if (e.key === "Escape") {
    closeMirrorModal();
    els.backupPreviewModal.classList.add("hidden");
  }
});

// 持久化开关偏好
const savedAutoClean = localStorage.getItem("condasrc.autoClean");
if (savedAutoClean === "true") {
  els.autoCleanToggle.checked = true;
}
els.autoCleanToggle.addEventListener("change", () => {
  localStorage.setItem("condasrc.autoClean", String(els.autoCleanToggle.checked));
});

const savedSyncRoot = localStorage.getItem("condasrc.syncRoot");
if (savedSyncRoot !== null) {
  els.syncRootToggle.checked = savedSyncRoot === "true";
}
els.syncRootToggle.addEventListener("change", () => {
  localStorage.setItem("condasrc.syncRoot", String(els.syncRootToggle.checked));
});

const savedSyncEnvMeta = localStorage.getItem("condasrc.syncEnvMeta");
if (savedSyncEnvMeta !== null) {
  els.syncEnvMetaToggle.checked = savedSyncEnvMeta === "true";
}
els.syncEnvMetaToggle.addEventListener("change", () => {
  localStorage.setItem("condasrc.syncEnvMeta", String(els.syncEnvMetaToggle.checked));
});

// 初始化加载
if (invoke) {
  loadSnapshot();
} else {
  showToast("未检测到 Tauri 运行时，请通过桌面程序启动", "error");
}
