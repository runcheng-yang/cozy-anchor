import { invoke } from "@tauri-apps/api/core";

async function init() {
  const app = document.querySelector("#app");

  const nodeId = await invoke("get_node_id").catch(() => "未启动");

  app.innerHTML = `
    <div class="container">
      <header>
        <h1>CozyAnchor</h1>
        <div class="node-id">本机 ID: <code>${nodeId}</code></div>
      </header>

      <nav class="tabs">
        <button class="tab-btn active" data-tab="devices">设备</button>
        <button class="tab-btn" data-tab="messages">消息</button>
        <button class="tab-btn" data-tab="todos">待办</button>
        <button class="tab-btn" data-tab="files">文件</button>
      </nav>

      <main>
        <div class="tab-panel active" id="tab-devices">
          <div class="panel-header">
            <h2>已配对设备</h2>
            <button id="btn-add-device">+ 添加设备</button>
          </div>
          <div id="devices-list" class="list"></div>
          <div id="add-device-form" class="hidden">
            <input type="text" id="input-node-id" placeholder="输入对方 NodeID" />
            <input type="text" id="input-device-name" placeholder="设备名称" />
            <button id="btn-confirm-pair">配对</button>
            <button id="btn-cancel-pair">取消</button>
          </div>
        </div>

        <div class="tab-panel" id="tab-messages">
          <div class="panel-header">
            <h2>消息</h2>
            <select id="msg-target-device">
              <option value="">选择目标设备</option>
            </select>
          </div>
          <div id="messages-list" class="list messages-list"></div>
          <div class="input-row">
            <input type="text" id="input-message" placeholder="输入消息..." />
            <button id="btn-send-msg">发送</button>
          </div>
        </div>

        <div class="tab-panel" id="tab-todos">
          <div class="panel-header">
            <h2>待办</h2>
            <select id="sync-target-device">
              <option value="">选择同步目标</option>
            </select>
            <button id="btn-sync">同步</button>
          </div>
          <div id="todos-list" class="list"></div>
          <div class="input-row">
            <input type="text" id="input-todo-title" placeholder="待办标题" />
            <button id="btn-add-todo">添加</button>
          </div>
        </div>

        <div class="tab-panel" id="tab-files">
          <div class="panel-header">
            <h2>文件传输</h2>
            <select id="file-target-device">
              <option value="">选择目标设备</option>
            </select>
          </div>
          <div class="file-actions">
            <button id="btn-send-file">选择文件发送</button>
            <button id="btn-set-receive-dir">设置接收文件夹</button>
          </div>
          <div id="receive-dir-info"></div>
          <div id="transfers-list" class="list"></div>
        </div>
      </main>
    </div>
  `;

  // Tabs
  document.querySelectorAll(".tab-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      document.querySelectorAll(".tab-btn").forEach((b) => b.classList.remove("active"));
      document.querySelectorAll(".tab-panel").forEach((p) => p.classList.remove("active"));
      btn.classList.add("active");
      document.getElementById(`tab-${btn.dataset.tab}`).classList.add("active");
    });
  });

  // Device pairing
  document.getElementById("btn-add-device").addEventListener("click", () => {
    document.getElementById("add-device-form").classList.remove("hidden");
  });
  document.getElementById("btn-cancel-pair").addEventListener("click", () => {
    document.getElementById("add-device-form").classList.add("hidden");
  });
  document.getElementById("btn-confirm-pair").addEventListener("click", async () => {
    const nodeId = document.getElementById("input-node-id").value.trim();
    const name = document.getElementById("input-device-name").value.trim();
    if (!nodeId) return;
    try {
      await invoke("pair_device", { nodeId, name: name || "未命名设备" });
      document.getElementById("add-device-form").classList.add("hidden");
      loadDevices();
    } catch (e) {
      alert("配对失败: " + e);
    }
  });

  // Messages
  document.getElementById("btn-send-msg").addEventListener("click", async () => {
    const content = document.getElementById("input-message").value.trim();
    const targetNode = document.getElementById("msg-target-device").value;
    if (!content || !targetNode) return;
    try {
      await invoke("send_message", { content, targetNode });
      document.getElementById("input-message").value = "";
      loadMessages();
    } catch (e) {
      alert("发送失败: " + e);
    }
  });

  // Todos
  document.getElementById("btn-add-todo").addEventListener("click", async () => {
    const title = document.getElementById("input-todo-title").value.trim();
    if (!title) return;
    try {
      await invoke("create_todo", { title });
      document.getElementById("input-todo-title").value = "";
      loadTodos();
    } catch (e) {
      alert("添加失败: " + e);
    }
  });

  document.getElementById("btn-sync").addEventListener("click", async () => {
    const targetNode = document.getElementById("sync-target-device").value;
    if (!targetNode) return;
    try {
      await invoke("sync_with_device", { targetNode });
      loadTodos();
      loadMessages();
      alert("同步完成");
    } catch (e) {
      alert("同步失败: " + e);
    }
  });

  // File
  document.getElementById("btn-send-file").addEventListener("click", async () => {
    const targetNode = document.getElementById("file-target-device").value;
    if (!targetNode) {
      alert("请先选择目标设备");
      return;
    }
    try {
      await invoke("send_file", { targetNode });
      loadTransfers();
    } catch (e) {
      alert("发送失败: " + e);
    }
  });

  document.getElementById("btn-set-receive-dir").addEventListener("click", async () => {
    try {
      await invoke("set_receive_dir");
      loadReceiveDir();
    } catch (e) {
      alert("设置失败: " + e);
    }
  });

  async function loadDevices() {
    const devices = await invoke("list_devices").catch(() => []);
    const list = document.getElementById("devices-list");
    list.innerHTML = devices.length
      ? devices.map((d) => `
          <div class="item device-item">
            <span class="device-name">${d.name}</span>
            <code class="device-id">${d.node_id.substring(0, 16)}...</code>
            <button class="btn-delete" data-id="${d.node_id}">删除</button>
          </div>
        `).join("")
      : `<p class="empty">暂无配对设备</p>`;

    // Update device selectors
    const opts = devices.map((d) => `<option value="${d.node_id}">${d.name}</option>`).join("");
    ["msg-target-device", "sync-target-device", "file-target-device"].forEach((id) => {
      const sel = document.getElementById(id);
      const current = sel.value;
      sel.innerHTML = '<option value="">选择设备</option>' + opts;
      sel.value = current;
    });

    list.querySelectorAll(".btn-delete").forEach((btn) => {
      btn.addEventListener("click", async () => {
        await invoke("remove_device", { nodeId: btn.dataset.id });
        loadDevices();
      });
    });
  }

  async function loadMessages() {
    const msgs = await invoke("list_messages").catch(() => []);
    const list = document.getElementById("messages-list");
    list.innerHTML = msgs.length
      ? msgs.map((m) => `
          <div class="msg-item">
            <div class="msg-meta">${new Date(m.created_at).toLocaleString()} ${m.device_id ? "(来自: " + m.device_id.substring(0, 8) + "...)" : ""}</div>
            <div class="msg-content">${m.content}</div>
          </div>
        `).join("")
      : `<p class="empty">暂无消息</p>`;
  }

  async function loadTodos() {
    const todos = await invoke("list_todos").catch(() => []);
    const list = document.getElementById("todos-list");
    list.innerHTML = todos.length
      ? todos.map((t) => `
          <div class="item todo-item ${t.status}">
            <input type="checkbox" ${t.status === "done" ? "checked" : ""} data-id="${t.id}" />
            <span class="todo-title">${t.title}</span>
            <button class="btn-delete" data-id="${t.id}">删除</button>
          </div>
        `).join("")
      : `<p class="empty">暂无待办</p>`;

    list.querySelectorAll('input[type="checkbox"]').forEach((cb) => {
      cb.addEventListener("change", async () => {
        await invoke("update_todo_status", { id: cb.dataset.id, status: cb.checked ? "done" : "pending" });
        loadTodos();
      });
    });

    list.querySelectorAll(".btn-delete").forEach((btn) => {
      btn.addEventListener("click", async () => {
        await invoke("delete_todo", { id: btn.dataset.id });
        loadTodos();
      });
    });
  }

  async function loadTransfers() {
    const transfers = await invoke("list_transfers").catch(() => []);
    const list = document.getElementById("transfers-list");
    list.innerHTML = transfers.length
      ? transfers.map((t) => `
          <div class="item transfer-item ${t.status}">
            <span class="transfer-name">${t.filename}</span>
            <span class="transfer-size">${formatSize(t.size_bytes)}</span>
            <span class="transfer-status">${t.status}</span>
          </div>
        `).join("")
      : `<p class="empty">暂无传输记录</p>`;
  }

  async function loadReceiveDir() {
    const dir = await invoke("get_receive_dir").catch(() => null);
    document.getElementById("receive-dir-info").textContent = dir ? `接收文件夹: ${dir}` : "未设置接收文件夹";
  }

  function formatSize(bytes) {
    if (!bytes) return "0 B";
    const units = ["B", "KB", "MB", "GB"];
    let i = 0;
    while (bytes >= 1024 && i < units.length - 1) { bytes /= 1024; i++; }
    return bytes.toFixed(1) + " " + units[i];
  }

  // Initial load
  loadDevices();
  loadMessages();
  loadTodos();
  loadTransfers();
  loadReceiveDir();

  // Refresh periodically
  setInterval(() => {
    loadDevices();
    loadMessages();
    loadTodos();
    loadTransfers();
  }, 3000);
}

init();
