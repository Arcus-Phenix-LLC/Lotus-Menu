'use strict';
const $ = (id) => document.getElementById(id);
const invoke = window.__TAURI__?.core.invoke;
let menu = { items: [], choices: [], warnings: [] };
let scanning = false;
function status(message) { $('status').textContent = message; }
function node(tag, className, text) {
  const el = document.createElement(tag);
  if (className) el.className = className;
  if (text !== undefined) el.textContent = text;
  return el;
}
function count(items) { return items.reduce((sum, item) => sum + (item.children ? count(item.children) : 1), 0); }
function panel(items, depth = 0) {
  const host = $('menus');
  while (host.children.length > depth) host.lastElementChild.remove();
  const list = node('ul', 'menu-panel');
  list.setAttribute('role', 'menu');
  list.setAttribute('aria-label', depth ? 'Submenu' : 'Programs');
  items.forEach((item, index) => {
    const li = node('li'); li.setAttribute('role', 'none');
    const button = node('button', 'menu-item');
    button.type = 'button'; button.setAttribute('role', 'menuitem');
    button.title = item.id;
    const icon = node('span', `menu-icon ${item.children ? '' : 'tool-icon'}`, item.children ? '▰' : '▣');
    icon.setAttribute('aria-hidden', 'true');
    button.append(icon, node('span', 'menu-name', item.name));
    if (item.children) {
      button.setAttribute('aria-haspopup', 'menu'); button.setAttribute('aria-expanded', 'false');
      button.append(node('span', 'arrow', '▶'));
    }
    function open() {
      list.querySelectorAll('[aria-expanded]').forEach(b => b.setAttribute('aria-expanded', 'false'));
      while (host.children.length > depth + 1) host.lastElementChild.remove();
      if (item.children) {
        button.setAttribute('aria-expanded', 'true');
        panel(item.children, depth + 1);
      }
    }
    button.addEventListener('mouseenter', open);
    button.addEventListener('click', async () => {
      if (item.children) { open(); return; }
      button.disabled = true;
      try { await invoke('launch_tool', { id: item.id }); status(`Started ${item.name}`); }
      catch (error) { status(`Could not launch ${item.name}: ${error}`); }
      finally { button.disabled = false; }
    });
    button.addEventListener('keydown', event => {
      const buttons = [...list.querySelectorAll('button')];
      if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
        event.preventDefault(); buttons[(index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length].focus();
      } else if (event.key === 'ArrowRight' && item.children) {
        event.preventDefault(); open(); host.children[depth + 1].querySelector('button')?.focus();
      } else if ((event.key === 'ArrowLeft' || event.key === 'Escape') && depth) {
        event.preventDefault();
        const parent = host.children[depth - 1].querySelector('[aria-expanded="true"]');
        while (host.children.length > depth) host.lastElementChild.remove();
        parent?.setAttribute('aria-expanded', 'false'); parent?.focus();
      } else if (event.key === 'Home' || event.key === 'End') {
        event.preventDefault(); buttons[event.key === 'Home' ? 0 : buttons.length - 1].focus();
      }
    });
    li.append(button); list.append(li);
  });
  host.append(list);
}
function renderChoices() {
  $('choices').replaceChildren();
  $('setup').hidden = menu.choices.length === 0;
  $('pending').textContent = `(${menu.choices.length})`;
  menu.choices.forEach(choice => {
    const box = node('article', 'choice');
    box.append(node('h3', '', choice.folder), node('p', '', choice.reason));
    const form = node('form');
    const nameLabel = node('label', '', 'Menu name');
    const name = node('input'); name.value = choice.name; name.required = true;
    nameLabel.append(name);
    const fileLabel = node('label', '', 'Launch file');
    const select = node('select');
    choice.candidates.forEach(path => { const option = node('option', '', path); option.value = path; select.append(option); });
    fileLabel.append(select);
    const save = node('button', '', 'Save choice'); save.type = 'submit';
    form.append(nameLabel, fileLabel, save);
    form.addEventListener('submit', async event => {
      event.preventDefault(); save.disabled = true;
      try {
        await invoke('save_choice', { folder: choice.folder, launch: select.value, name: name.value.trim() });
        await refresh();
      } catch (error) { status(`Could not save choice: ${error}`); }
      finally { save.disabled = false; }
    });
    box.append(form); $('choices').append(box);
  });
}
async function refresh() {
  if (scanning) return;
  scanning = true; $('refresh').disabled = true; status('Scanning tools…');
  try {
    const next = await invoke('scan_directory'); menu = next;
    $('menus').replaceChildren();
    $('menus').hidden = menu.items.length === 0;
    $('programs').setAttribute('aria-expanded', String(menu.items.length > 0));
    if (menu.items.length) panel(menu.items);
    $('empty').hidden = menu.items.length > 0 || menu.choices.length > 0;
    renderChoices();
    $('warnings').replaceChildren(...menu.warnings.map(text => node('p', '', text)));
    $('warnings').hidden = !menu.warnings.length;
    status(`${count(menu.items)} tools ready · ${menu.choices.length} awaiting a launch choice${menu.warnings.length ? ` · ${menu.warnings.length} warnings` : ''}`);
  } catch (error) { status(`Could not scan tools: ${error}`); }
  finally { scanning = false; $('refresh').disabled = false; }
}
async function browse() {
  try { await invoke('open_folder'); status('Opened tools folder. Select Refresh Menu after making changes.'); }
  catch (error) { status(`Could not open folder: ${error}`); }
}
$('refresh').addEventListener('click', refresh);
$('browse').addEventListener('click', browse);
$('empty-browse').addEventListener('click', browse);
$('about').addEventListener('click', () => $('help').showModal());
$('programs').addEventListener('click', () => {
  $('menus').hidden = !$('menus').hidden;
  $('programs').setAttribute('aria-expanded', String(!$('menus').hidden));
  if (!$('menus').hidden) $('menus').querySelector('button')?.focus();
});
if (invoke) refresh();
else {
  status('Browser preview only. Run the Tauri app to scan and launch your tools.');
  $('refresh').disabled = $('browse').disabled = $('empty-browse').disabled = true;
  $('empty').hidden = false;
  $('menus').hidden = true;
}
