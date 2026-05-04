// ══════════════════════════════════════════════════════════════════════════
// IndOS — Conversation Canvas Engine
// ══════════════════════════════════════════════════════════════════════════

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

// ── State ─────────────────────────────────────────────────────────────────

const state = {
  threads: [],
  activeThread: null,
  messages: [],
  isStreaming: false,
  ollamaOnline: false,
  selectedModel: 'llama3.2',
  sidebarOpen: true,
};

// ── DOM References ────────────────────────────────────────────────────────

const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

const dom = {
  statusDot: $('#status-indicator'),
  activeModel: $('#active-model'),
  sidebar: $('#sidebar'),
  threadList: $('#thread-list'),
  welcome: $('#welcome-screen'),
  messagesContainer: $('#messages-container'),
  messages: $('#messages'),
  messageInput: $('#message-input'),
  sendBtn: $('#send-btn'),
  modelSelect: $('#model-select'),
  systemMini: $('#system-info-mini'),
};

// ── Initialize ────────────────────────────────────────────────────────────

document.addEventListener('DOMContentLoaded', async () => {
  setupWindowControls();
  setupInputHandlers();
  setupSuggestionChips();
  setupSidebarToggle();

  await checkOllamaStatus();
  await loadModels();
  await loadThreads();
  await loadSystemInfo();

  // Set up streaming event listeners
  setupStreamListeners();

  // Auto-refresh status every 15s
  setInterval(checkOllamaStatus, 15000);
});

// ── Window Controls ───────────────────────────────────────────────────────

function setupWindowControls() {
  const appWindow = getCurrentWindow();

  $('#btn-minimize')?.addEventListener('click', () => appWindow.minimize());
  $('#btn-maximize')?.addEventListener('click', async () => {
    const maximized = await appWindow.isMaximized();
    maximized ? appWindow.unmaximize() : appWindow.maximize();
  });
  $('#btn-close')?.addEventListener('click', () => appWindow.close());
}

// ── Ollama Status ─────────────────────────────────────────────────────────

async function checkOllamaStatus() {
  dom.statusDot.className = 'status-dot checking';
  dom.statusDot.title = 'Checking Ollama...';

  try {
    const online = await invoke('check_ollama_status');
    state.ollamaOnline = online;
    dom.statusDot.className = `status-dot ${online ? 'online' : 'offline'}`;
    dom.statusDot.title = online ? 'Ollama connected' : 'Ollama offline — start with: ollama serve';

    if (online) {
      dom.activeModel.textContent = state.selectedModel;
    } else {
      dom.activeModel.textContent = 'ollama offline';
    }
  } catch (e) {
    state.ollamaOnline = false;
    dom.statusDot.className = 'status-dot offline';
    dom.statusDot.title = 'Cannot reach Ollama';
    dom.activeModel.textContent = 'disconnected';
  }
}

// ── Models ────────────────────────────────────────────────────────────────

async function loadModels() {
  try {
    const models = await invoke('list_models');
    dom.modelSelect.innerHTML = '';

    if (models.length === 0) {
      dom.modelSelect.innerHTML = '<option value="">No models found</option>';
      return;
    }

    models.forEach((m) => {
      const opt = document.createElement('option');
      opt.value = m.name;
      const sizeMB = m.size ? `${(m.size / 1e9).toFixed(1)}GB` : '';
      opt.textContent = `${m.name} ${sizeMB}`;
      dom.modelSelect.appendChild(opt);
    });

    // Select first model
    if (models.length > 0) {
      state.selectedModel = models[0].name;
      dom.modelSelect.value = state.selectedModel;
    }
  } catch (e) {
    dom.modelSelect.innerHTML = '<option value="llama3.2">llama3.2 (default)</option>';
  }

  dom.modelSelect.addEventListener('change', (e) => {
    state.selectedModel = e.target.value;
    dom.activeModel.textContent = state.selectedModel;
  });
}

// ── Threads ───────────────────────────────────────────────────────────────

async function loadThreads() {
  try {
    state.threads = await invoke('list_threads');
  } catch (e) {
    state.threads = [];
  }
  renderThreadList();
}

function renderThreadList() {
  dom.threadList.innerHTML = '';

  if (state.threads.length === 0) {
    dom.threadList.innerHTML = `
      <div style="padding: 20px; text-align: center; color: var(--text-tertiary); font-size: 12px;">
        No conversations yet.<br>Start typing below.
      </div>
    `;
    return;
  }

  state.threads.forEach((thread) => {
    const el = document.createElement('div');
    el.className = `thread-item ${thread.id === state.activeThread ? 'active' : ''}`;
    el.innerHTML = `
      <span class="thread-title">${escapeHtml(thread.title)}</span>
      <span class="thread-time">${formatTime(thread.updated_at)}</span>
      <button class="thread-delete" title="Delete">×</button>
    `;

    el.addEventListener('click', (e) => {
      if (e.target.classList.contains('thread-delete')) {
        deleteThread(thread.id);
        return;
      }
      switchThread(thread.id);
    });

    dom.threadList.appendChild(el);
  });
}

async function createNewThread(firstMessage) {
  try {
    const title = firstMessage.slice(0, 50) + (firstMessage.length > 50 ? '...' : '');
    const thread = await invoke('create_thread', { title });
    state.threads.unshift(thread);
    state.activeThread = thread.id;
    renderThreadList();
    return thread;
  } catch (e) {
    console.error('Failed to create thread:', e);
    return null;
  }
}

async function switchThread(threadId) {
  state.activeThread = threadId;
  renderThreadList();

  try {
    state.messages = await invoke('get_thread_messages', { threadId });
    renderMessages();
    showMessages();
  } catch (e) {
    console.error('Failed to load messages:', e);
  }
}

async function deleteThread(threadId) {
  try {
    await invoke('delete_thread', { threadId });
    state.threads = state.threads.filter((t) => t.id !== threadId);
    if (state.activeThread === threadId) {
      state.activeThread = null;
      state.messages = [];
      showWelcome();
    }
    renderThreadList();
  } catch (e) {
    console.error('Failed to delete thread:', e);
  }
}

$('#btn-new-thread')?.addEventListener('click', () => {
  state.activeThread = null;
  state.messages = [];
  showWelcome();
  renderThreadList();
  dom.messageInput.focus();
});

// ── Messages ──────────────────────────────────────────────────────────────

function showWelcome() {
  dom.welcome.classList.remove('hidden');
  dom.messagesContainer.classList.add('hidden');
}

function showMessages() {
  dom.welcome.classList.add('hidden');
  dom.messagesContainer.classList.remove('hidden');
}

function renderMessages() {
  dom.messages.innerHTML = '';
  state.messages.forEach((msg) => {
    dom.messages.appendChild(createMessageElement(msg));
  });
  scrollToBottom();
}

function createMessageElement(msg) {
  const el = document.createElement('div');
  el.className = 'message';
  el.id = `msg-${msg.id}`;

  const isUser = msg.role === 'user';
  const avatar = isUser ? '→' : '◆';
  const roleName = isUser ? 'You' : 'IndOS';
  const modelTag = msg.model ? `<span class="message-model">${msg.model}</span>` : '';

  el.innerHTML = `
    <div class="message-header">
      <div class="message-avatar ${msg.role}">${avatar}</div>
      <span class="message-role">${roleName}</span>
      ${modelTag}
    </div>
    <div class="message-body">
      <div class="message-content">${renderMarkdown(msg.content)}</div>
      ${renderFragments(msg.fragments)}
    </div>
  `;

  return el;
}

// ── Send Message ──────────────────────────────────────────────────────────

async function sendMessage(content) {
  if (!content.trim() || state.isStreaming) return;

  // Ensure we have a thread
  if (!state.activeThread) {
    const thread = await createNewThread(content);
    if (!thread) return;
  }

  state.isStreaming = true;
  dom.sendBtn.disabled = true;
  showMessages();

  // Render user message immediately
  const userMsg = {
    id: crypto.randomUUID(),
    role: 'user',
    content: content,
    fragments: [],
    timestamp: new Date().toISOString(),
    model: null,
  };
  state.messages.push(userMsg);
  dom.messages.appendChild(createMessageElement(userMsg));

  // Add streaming placeholder
  const streamId = crypto.randomUUID();
  const streamEl = document.createElement('div');
  streamEl.className = 'message';
  streamEl.id = `msg-${streamId}`;
  streamEl.innerHTML = `
    <div class="message-header">
      <div class="message-avatar assistant">◆</div>
      <span class="message-role">IndOS</span>
      <span class="message-model">${state.selectedModel}</span>
    </div>
    <div class="message-body">
      <div class="message-content" id="stream-content-${streamId}">
        <span class="streaming-indicator">
          <span class="dot"></span>
          <span class="dot"></span>
          <span class="dot"></span>
        </span>
      </div>
    </div>
  `;
  dom.messages.appendChild(streamEl);
  scrollToBottom();

  try {
    const msgId = await invoke('send_message_streaming', {
      threadId: state.activeThread,
      content: content,
      model: state.selectedModel,
    });

    // Store stream target for event listeners
    state.currentStreamId = msgId;
    state.currentStreamElement = streamId;
    state.streamContent = '';
  } catch (e) {
    // Fallback to non-streaming
    try {
      const response = await invoke('send_message', {
        threadId: state.activeThread,
        content: content,
      });

      // Remove stream placeholder, add real response
      streamEl.remove();
      state.messages.push(response);
      dom.messages.appendChild(createMessageElement(response));
    } catch (e2) {
      const errorContent = document.getElementById(`stream-content-${streamId}`);
      if (errorContent) {
        errorContent.innerHTML = `<span style="color: var(--error);">⚠ ${escapeHtml(e2.toString())}</span>`;
      }
    }
    state.isStreaming = false;
    dom.sendBtn.disabled = false;
  }

  scrollToBottom();
  dom.messageInput.value = '';
  dom.messageInput.style.height = 'auto';
}

// ── Stream Event Listeners ────────────────────────────────────────────────

function setupStreamListeners() {
  listen('stream-chunk', (event) => {
    const { msgId, chunk, done } = event.payload;
    if (state.currentStreamId !== msgId) return;

    state.streamContent += chunk;
    const el = document.getElementById(`stream-content-${state.currentStreamElement}`);
    if (el) {
      el.innerHTML = renderMarkdown(state.streamContent);
      if (!done) {
        el.innerHTML += `<span class="streaming-indicator"><span class="dot"></span><span class="dot"></span><span class="dot"></span></span>`;
      }
      scrollToBottom();
    }
  });

  listen('stream-complete', (event) => {
    const { msgId, content, fragments, model } = event.payload;
    if (state.currentStreamId !== msgId) return;

    const el = document.getElementById(`stream-content-${state.currentStreamElement}`);
    if (el) {
      el.innerHTML = renderMarkdown(content);
      // Add fragment rendering
      const parent = el.closest('.message-body');
      if (parent && fragments && fragments.length > 0) {
        parent.innerHTML += renderFragments(fragments);
      }
    }

    state.messages.push({
      id: msgId,
      role: 'assistant',
      content,
      fragments: fragments || [],
      timestamp: new Date().toISOString(),
      model,
    });

    state.isStreaming = false;
    dom.sendBtn.disabled = false;
    state.streamContent = '';
    scrollToBottom();
  });

  listen('stream-error', (event) => {
    const { msgId, error } = event.payload;
    if (state.currentStreamId !== msgId) return;

    const el = document.getElementById(`stream-content-${state.currentStreamElement}`);
    if (el) {
      el.innerHTML = `<span style="color: var(--error);">⚠ ${escapeHtml(error)}</span>`;
    }

    state.isStreaming = false;
    dom.sendBtn.disabled = false;
    state.streamContent = '';
  });
}

// ── Input Handlers ────────────────────────────────────────────────────────

function setupInputHandlers() {
  dom.messageInput.addEventListener('keydown', (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      sendMessage(dom.messageInput.value);
    }
  });

  // Auto-resize textarea
  dom.messageInput.addEventListener('input', () => {
    dom.messageInput.style.height = 'auto';
    dom.messageInput.style.height = Math.min(dom.messageInput.scrollHeight, 200) + 'px';
  });

  dom.sendBtn.addEventListener('click', () => {
    sendMessage(dom.messageInput.value);
  });
}

// ── Suggestion Chips ──────────────────────────────────────────────────────

function setupSuggestionChips() {
  $$('.suggestion-chip').forEach((chip) => {
    chip.addEventListener('click', () => {
      const prompt = chip.dataset.prompt;
      dom.messageInput.value = prompt;
      sendMessage(prompt);
    });
  });
}

// ── Sidebar Toggle ────────────────────────────────────────────────────────

function setupSidebarToggle() {
  $('#btn-sidebar-toggle')?.addEventListener('click', () => {
    state.sidebarOpen = !state.sidebarOpen;
    dom.sidebar.classList.toggle('collapsed', !state.sidebarOpen);
  });
}

// ── System Info ───────────────────────────────────────────────────────────

async function loadSystemInfo() {
  try {
    const info = await invoke('get_system_info');
    const memPct = ((info.usedMemory / info.totalMemory) * 100).toFixed(0);
    const memGB = (info.usedMemory / 1e9).toFixed(1);
    const totalGB = (info.totalMemory / 1e9).toFixed(1);

    dom.systemMini.innerHTML = `
      <div>${info.hostname} · ${info.os}</div>
      <div>RAM: ${memGB}/${totalGB} GB (${memPct}%)</div>
      <div>CPUs: ${info.cpus} · Procs: ${info.processes}</div>
    `;
  } catch (e) {
    dom.systemMini.textContent = 'System info unavailable';
  }
}

// ── Fragment Renderer ─────────────────────────────────────────────────────

function renderFragments(fragments) {
  if (!fragments || fragments.length === 0) return '';

  return fragments
    .map((frag) => {
      const actions = (frag.actions || [])
        .map((a) => `<button class="fragment-action-btn" onclick="handleFragmentAction('${a.handler}', ${JSON.stringify(a.args || {})})">${escapeHtml(a.label)}</button>`)
        .join('');

      let body = '';

      switch (frag.component) {
        case 'file-browser':
          body = renderFileBrowser(frag.props);
          break;
        case 'system-monitor':
          body = renderSystemMonitor(frag.props);
          break;
        case 'code-block':
          body = renderCodeBlock(frag.props);
          break;
        default:
          body = `<pre style="font-size: 12px; color: var(--text-secondary);">${escapeHtml(JSON.stringify(frag.props, null, 2))}</pre>`;
      }

      return `
        <div class="fragment-container">
          <div class="fragment-header">
            <span class="fragment-type-badge">${frag.fragment_type}:${frag.component}</span>
            <div class="fragment-actions">${actions}</div>
          </div>
          <div class="fragment-body">${body}</div>
        </div>
      `;
    })
    .join('');
}

function renderFileBrowser(props) {
  if (!props.items || !Array.isArray(props.items)) return '<p>No files to display</p>';

  const items = props.items
    .map((item) => {
      const icon = item.isDir ? '📁' : getFileIcon(item.extension);
      const size = item.isDir ? '' : formatSize(item.size);
      return `
        <div class="file-item" onclick="handleFileClick('${escapeHtml(item.path)}', ${item.isDir})">
          <span class="file-icon">${icon}</span>
          <span class="file-name">${escapeHtml(item.name)}</span>
          <span class="file-size">${size}</span>
        </div>
      `;
    })
    .join('');

  return `<div class="file-grid">${items}</div>`;
}

function renderSystemMonitor(props) {
  const cards = Object.entries(props)
    .map(([key, value]) => {
      if (typeof value === 'object') return '';
      return `
        <div class="sysinfo-card">
          <div class="sysinfo-label">${key.replace(/([A-Z])/g, ' $1').trim()}</div>
          <div class="sysinfo-value">${value}</div>
        </div>
      `;
    })
    .join('');

  return `<div class="sysinfo-grid">${cards}</div>`;
}

function renderCodeBlock(props) {
  return `<pre><code>${escapeHtml(props.code || JSON.stringify(props, null, 2))}</code></pre>`;
}

// ── Fragment Action Handlers ──────────────────────────────────────────────

window.handleFragmentAction = function (handler, args) {
  console.log('Fragment action:', handler, args);
  // Future: route to agent or system command
};

window.handleFileClick = async function (path, isDir) {
  if (isDir) {
    // Navigate into directory
    dom.messageInput.value = `Show me the files in ${path}`;
    sendMessage(dom.messageInput.value);
  } else {
    // Preview file
    dom.messageInput.value = `Show me the contents of ${path}`;
    sendMessage(dom.messageInput.value);
  }
};

// ── Markdown Renderer (Lightweight) ───────────────────────────────────────

function renderMarkdown(text) {
  if (!text) return '';

  let html = escapeHtml(text);

  // Code blocks (``` ... ```)
  html = html.replace(/```(\w*)\n([\s\S]*?)```/g, (_, lang, code) => {
    return `<pre><code class="language-${lang}">${code.trim()}</code></pre>`;
  });

  // Skip further processing inside code blocks
  const codeBlocks = [];
  html = html.replace(/<pre><code[^>]*>[\s\S]*?<\/code><\/pre>/g, (match) => {
    codeBlocks.push(match);
    return `__CODE_BLOCK_${codeBlocks.length - 1}__`;
  });

  // Inline code
  html = html.replace(/`([^`]+)`/g, '<code>$1</code>');

  // Bold
  html = html.replace(/\*\*([^*]+)\*\*/g, '<strong>$1</strong>');

  // Italic
  html = html.replace(/\*([^*]+)\*/g, '<em>$1</em>');

  // Headers
  html = html.replace(/^### (.+)$/gm, '<h3>$1</h3>');
  html = html.replace(/^## (.+)$/gm, '<h2>$1</h2>');
  html = html.replace(/^# (.+)$/gm, '<h1>$1</h1>');

  // Lists
  html = html.replace(/^- (.+)$/gm, '<li>$1</li>');
  html = html.replace(/^(\d+)\. (.+)$/gm, '<li>$2</li>');

  // Links
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank">$1</a>');

  // Line breaks → paragraphs
  html = html.replace(/\n\n/g, '</p><p>');
  html = html.replace(/\n/g, '<br>');
  html = '<p>' + html + '</p>';

  // Clean up empty paragraphs
  html = html.replace(/<p>\s*<\/p>/g, '');
  html = html.replace(/<p><(h[123]|pre|li)/g, '<$1');
  html = html.replace(/<\/(h[123]|pre|li)><\/p>/g, '</$1>');

  // Restore code blocks
  codeBlocks.forEach((block, i) => {
    html = html.replace(`__CODE_BLOCK_${i}__`, block);
  });

  return html;
}

// ── Utilities ─────────────────────────────────────────────────────────────

function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function formatTime(isoStr) {
  try {
    const date = new Date(isoStr);
    const now = new Date();
    const diff = now - date;

    if (diff < 60000) return 'now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h`;
    return date.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  } catch {
    return '';
  }
}

function formatSize(bytes) {
  if (!bytes) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1048576) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1073741824) return `${(bytes / 1048576).toFixed(1)} MB`;
  return `${(bytes / 1073741824).toFixed(1)} GB`;
}

function getFileIcon(ext) {
  const icons = {
    js: '📜', ts: '📜', py: '🐍', rs: '🦀', go: '🔵', java: '☕',
    html: '🌐', css: '🎨', json: '📋', yaml: '📋', yml: '📋', toml: '📋',
    md: '📝', txt: '📄', pdf: '📕',
    png: '🖼️', jpg: '🖼️', jpeg: '🖼️', gif: '🖼️', svg: '🖼️', webp: '🖼️',
    mp4: '🎬', mkv: '🎬', avi: '🎬', mov: '🎬',
    mp3: '🎵', wav: '🎵', flac: '🎵', ogg: '🎵',
    zip: '📦', tar: '📦', gz: '📦', '7z': '📦',
    sh: '⚙️', bash: '⚙️', zsh: '⚙️',
    lock: '🔒', env: '🔐',
  };
  return icons[ext?.toLowerCase()] || '📄';
}

function scrollToBottom() {
  requestAnimationFrame(() => {
    dom.messagesContainer.scrollTop = dom.messagesContainer.scrollHeight;
  });
}
