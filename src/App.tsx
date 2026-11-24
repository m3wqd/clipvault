import { useEffect, useRef, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Clipboard, History, Search, Pin, Copy, Trash2, Pause, Play, FileText, Link, Code, Image as ImageIcon, Star, Folder, Settings, SlidersHorizontal, ChevronDown, ShieldCheck, ArrowDownWideNarrow } from 'lucide-react';
import { useTranslation } from './i18n';
import { SettingsPage, SnippetsPage, CollectionsPage } from './pages/Tools';
import { ItemExtras, ImagePreview } from './components/ItemExtras';
import { request } from './services/ipc';
import type { Item, Named, Status } from './types';

const kindNames = { text: 'Text', code: 'Code', url: 'Link', image: 'Image' } as const;
function ContentIcon({ kind }: { kind: Item['kind'] }) {
  const Icon = kind === 'url' ? Link : kind === 'code' ? Code : kind === 'image' ? ImageIcon : FileText;
  return <Icon size={17} aria-hidden="true" />;
}

export function App() {
  const { t, language, shortcut } = useTranslation();
  const [page, setPage] = useState('history');
  const [favorite, setFavorite] = useState(false);
  const [collection, setCollection] = useState<number | null>(null);
  const [collections, setCollections] = useState<Named[]>([]);
  const [tag, setTag] = useState('');
  const [frequent, setFrequent] = useState(false);
  const [since, setSince] = useState(0);
  const [offset, setOffset] = useState(0);
  const [items, setItems] = useState<Item[]>([]);
  const [query, setQuery] = useState('');
  const [pinned, setPinned] = useState(false);
  const [kind, setKind] = useState('all');
  const [selected, setSelected] = useState<number | null>(null);
  const [status, setStatus] = useState<Status>({ paused: false, backend: 'Connecting…', error: null });
  const [error, setError] = useState('');
  const [loading, setLoading] = useState(true);
  const [revision, setRevision] = useState(0);
  const [notice, setNotice] = useState('');
  const [showFilters, setShowFilters] = useState(false);
  const sequence = useRef(0);
  const search = useRef<HTMLInputElement>(null);
  const noticeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const refresh = () => setRevision(v => v + 1);

  useEffect(() => {
    let alive = true;
    void request<Named[]>({ action: 'collections' }).then(rows => { if (alive) setCollections(rows); }).catch(() => {});
    return () => { alive = false; };
  }, [page, revision]);
  useEffect(() => setOffset(0), [query, pinned, kind, favorite, collection, tag, since, frequent]);
  useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k' && page === 'history') {
        event.preventDefault(); search.current?.focus();
      }
    };
    window.addEventListener('keydown', focusSearch);
    return () => window.removeEventListener('keydown', focusSearch);
  }, [page]);
  useEffect(() => () => clearTimeout(noticeTimer.current), []);
  useEffect(() => {
    let disposed = false;
    const cleanups: Array<() => void> = [];
    const loadStatus = () => {
      void request<Status>({ action: 'status' }).then(s => { if (!disposed) setStatus(s); })
        .catch(e => { if (!disposed) setError(String(e)); });
    };
    const add = async () => {
      for (const [event, fn] of [['history-changed', refresh], ['tracking-status', loadStatus]] as const) {
        const off = await listen(event, fn);
        if (disposed) off(); else cleanups.push(off);
      }
    };
    void add().catch(e => { if (!disposed) setError(String(e)); });
    loadStatus();
    return () => { disposed = true; cleanups.forEach(f => f()); };
  }, []);
  useEffect(() => {
    const seq = ++sequence.current;
    setLoading(true);
    const timer = setTimeout(() => {
      void request<Item[]>({ action: 'list', query, pinned, kind, favorite, collection, tag, since, frequent, offset })
        .then(rows => {
          if (seq === sequence.current) {
            setItems(rows); setError('');
            setSelected(id => rows.some(x => x.id === id) ? id : rows[0]?.id ?? null);
          }
        }).catch(e => { if (seq === sequence.current) setError(String(e)); })
        .finally(() => { if (seq === sequence.current) setLoading(false); });
    }, 180);
    return () => { clearTimeout(timer); sequence.current++; };
  }, [query, pinned, kind, favorite, collection, tag, since, frequent, offset, revision]);

  const item = items.find(i => i.id === selected);
  const hasFilters = Boolean(collection || tag || since || frequent);
  const collectionName = collections.find(c => c.id === collection)?.name;
  const title = collectionName ?? (favorite ? t('Favorites') : pinned ? t('Pinned') : t('History'));
  const locale = language === 'ru' ? 'ru-RU' : 'en-GB';
  const shortDate = (value: number) => {
    const date = new Date(value);
    return date.toDateString() === new Date().toDateString()
      ? date.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' })
      : date.toLocaleDateString(locale, { day: 'numeric', month: 'short' });
  };
  async function action(fn: () => Promise<unknown>) {
    try { await fn(); } catch (e) { setError(String(e)); }
  }
  const copy = (id: number) => action(async () => {
    await request({ action: 'copy', id });
    setNotice(t('Copied to clipboard'));
    clearTimeout(noticeTimer.current);
    noticeTimer.current = setTimeout(() => setNotice(''), 2500);
  });
  const pin = (i: Item) => action(() => request({ action: 'pin', id: i.id, value: !i.pinned }));
  const remove = (id: number) => action(() => request({ action: 'delete', id }));
  function navigate(view: 'all' | 'pinned' | 'favorites', id: number | null = null) {
    setPage('history'); setPinned(view === 'pinned'); setFavorite(view === 'favorites'); setCollection(id);
  }
  function moveSelection(event: React.KeyboardEvent) {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    const index = items.findIndex(i => i.id === selected);
    const nextIndex = Math.max(0, Math.min(items.length - 1, index + (event.key === 'ArrowDown' ? 1 : -1)));
    setSelected(items[nextIndex]?.id ?? null);
    event.currentTarget.querySelectorAll<HTMLButtonElement>('.row-body')[nextIndex]?.focus();
  }

  return <div className="shell">
    <aside className="sidebar">
      <div className="brand"><Clipboard size={19} aria-hidden="true" /><span>ClipVault</span></div>
      <nav aria-label={t('Navigation')}>
        <button className={page === 'history' && !pinned && !favorite && collection === null ? 'active' : ''} onClick={() => navigate('all')}><History size={17} />{t('History')}</button>
        <button className={page === 'history' && pinned ? 'active' : ''} onClick={() => navigate('pinned')}><Pin size={17} />{t('Pinned')}</button>
        <button className={page === 'history' && favorite ? 'active' : ''} onClick={() => navigate('favorites')}><Star size={17} />{t('Favorites')}</button>
        <button className={page === 'snippets' ? 'active' : ''} onClick={() => setPage('snippets')}><FileText size={17} />{t('Snippets')}</button>
      </nav>
      <div className="sidebar-collections">
        <button className="collection-heading" onClick={() => setPage('collections')}>{t('Collections')}<ChevronDown size={13} /></button>
        {collections.map(c => <button key={c.id} className={page === 'history' && collection === c.id ? 'active' : ''} onClick={() => navigate('all', c.id)}><Folder size={15} /><span>{c.name}</span></button>)}
        {!collections.length && <button className="add-collection" onClick={() => setPage('collections')}>+ {t('Create collection')}</button>}
      </div>
      <div className="sidebar-bottom">
        <button className={page === 'settings' ? 'settings-link active' : 'settings-link'} onClick={() => setPage('settings')}><Settings size={16} />{t('Settings')}</button>
        <div className="tracking" title={status.backend}><span className={status.incognito || status.paused ? 'dot paused' : 'dot'} /><span>{status.incognito ? t('Incognito') : status.paused ? t('Tracking paused') : status.backend === 'Connecting…' ? t('Connecting…') : status.error ? t('Tracking unavailable') : t('Recording on')}</span>
          <button aria-label={status.paused ? 'Resume tracking' : 'Pause tracking'} title={status.paused ? t('Resume tracking') : t('Pause tracking')} onClick={() => void action(async () => setStatus(await request<Status>({ action: 'pause', value: !status.paused })))}>{status.paused ? <Play size={14} /> : <Pause size={14} />}</button>
        </div>
      </div>
    </aside>
    <main className="main-area">
      {page === 'settings' ? <SettingsPage /> : page === 'snippets' ? <SnippetsPage /> : page === 'collections' ? <CollectionsPage /> : <>
        <header className="main-header"><h1>{title}</h1><label className="search"><Search size={16} /><input ref={search} aria-label="Search history" placeholder={t('Search')} value={query} onChange={e => setQuery(e.target.value)} /><span className="search-shortcut">Ctrl K</span></label></header>
        <div className="privacy"><ShieldCheck size={13} /><span>{t('History may contain secrets. Storage is not encrypted.')}</span></div>
        <div className="filters">{['all', 'text', 'url', 'code', 'image'].map(k => <button key={k} className={kind === k ? 'chosen' : ''} onClick={() => setKind(k)}>{k === 'all' ? t('All items') : k === 'url' ? t('Links') : k === 'code' ? t('Code') : k === 'image' ? t('Images') : t('Text')}</button>)}
          <button className={showFilters || hasFilters ? 'filter-toggle chosen' : 'filter-toggle'} aria-expanded={showFilters} aria-controls="advanced-filters" aria-label={t('Filters')} title={t('Filters')} onClick={() => setShowFilters(v => !v)}><SlidersHorizontal size={15} /></button>
        </div>
        {showFilters && <div className="advanced-filters" id="advanced-filters">
          <label>{t('Collection')}<select aria-label="Collection" value={collection ?? ''} onChange={e => setCollection(e.target.value ? Number(e.target.value) : null)}><option value="">{t('All collections')}</option>{collections.map(c => <option value={c.id} key={c.id}>{c.name}</option>)}</select></label>
          <label>{t('Tag')}<input aria-label="Filter tag" placeholder={t('Filter by tag')} value={tag} onChange={e => setTag(e.target.value)} /></label>
          <label>{t('Sort')}<select aria-label="Sort" value={frequent ? 'frequent' : 'recent'} onChange={e => setFrequent(e.target.value === 'frequent')}><option value="recent">{t('Most recent')}</option><option value="frequent">{t('Most copied')}</option></select></label>
          <label>{t('Time range')}<select aria-label="Time range" onChange={e => setSince(e.target.value ? Date.now() - Number(e.target.value) * 86400000 : 0)}><option value="">{t('All time')}</option><option value="1">{t('Last day')}</option><option value="7">{t('Last week')}</option><option value="30">{t('Last month')}</option></select></label>
        </div>}
        {error && <div role="alert" className="error">{error}</div>}
        {status.error && <div role="status" className="error">{status.error}</div>}
        <div className="content">
          <section className="history" aria-label="History" onKeyDown={moveSelection}>
            <div className="history-heading"><span>{query ? t('Search results') : t('Recent items')}</span><button aria-label={t('Change sort order')} title={frequent ? t('Most copied') : t('Most recent')} onClick={() => setFrequent(v => !v)}><ArrowDownWideNarrow size={15} /></button></div>
            <div className="history-list" aria-busy={loading}>
              {loading && <div className="hint">{t('Loading history…')}</div>}
              {!loading && !items.length && !error && <div className="empty"><Clipboard size={27} /><h2>{query ? t('No matching items') : t('Your clipboard starts here')}</h2><p>{t('Copy text or an image in another application.')}</p></div>}
              {items.map(i => <article key={i.id} className={selected === i.id ? 'history-row selected' : 'history-row'}>
                <button className="row-body" aria-pressed={selected === i.id} onClick={() => setSelected(i.id)} onDoubleClick={() => void copy(i.id)} onKeyDown={e => { if (e.key === 'Enter' && selected === i.id) { e.preventDefault(); void copy(i.id); } }}>
                  <span className="row-icon">{i.kind === 'image' ? <ImagePreview id={i.id} thumbnail /> : <ContentIcon kind={i.kind} />}</span><span className="row-content"><span className="row-title">{i.kind === 'image' ? t('Clipboard image') : i.text.replace(/\s+/g, ' ').slice(0, 180)}</span><span className="row-meta">{t(kindNames[i.kind])}{i.source ? ` · ${i.source}` : ''}{i.pinned ? ` · ${t('Pinned').toLowerCase()}` : ''}{frequent ? ` · ${i.copy_count}×` : ''}</span></span>
                  <time dateTime={new Date(i.last_copied).toISOString()}>{shortDate(i.last_copied)}</time>
                </button>

              </article>)}
            </div>
            <div className="history-footer"><span>{items.length}{items.length === 200 ? '+' : ''} {t('items')}</span><div className="pagination"><button disabled={offset === 0} onClick={() => setOffset(n => Math.max(0, n - 200))}>{t('Previous')}</button>{offset > 0 && <span>{offset / 200 + 1}</span>}<button disabled={items.length < 200} onClick={() => setOffset(n => n + 200)}>{t('Next')}</button></div><span className="popup-shortcut">{shortcut}</span></div>
          </section>
          <section className="preview" aria-label="Preview">
            <div className="preview-title"><span>{item ? t(kindNames[item.kind]) : t('Preview')}</span>{item && <div className="preview-tools"><button aria-label={item.pinned ? 'Unpin item' : 'Pin item'} title={item.pinned ? t('Unpin item') : t('Pin for later')} className={item.pinned ? 'is-pinned' : ''} onClick={() => void pin(item)}><Pin size={15} /></button><button aria-label={t('Favorite selected item')} title={t('Favorites')} className={item.favorite ? 'is-pinned' : ''} onClick={() => void action(() => request({ action: 'favorite', id: item.id, value: !item.favorite }))}><Star size={15} fill={item.favorite ? 'currentColor' : 'none'} /></button></div>}</div>
            {item ? <>
              <div className="preview-content">{item.kind === 'image' ? <ImagePreview id={item.id} /> : <pre>{item.text}</pre>}</div>
              <div className="preview-metadata"><span>{item.source ?? collectionName ?? t('Clipboard')}</span><time>{new Date(item.last_copied).toLocaleString(locale)}</time></div>
              <div className="preview-actions"><button className="primary" aria-label="Copy item" onClick={() => void copy(item.id)}><Copy size={15} />{t('Copy')}</button><button className="delete-selected" aria-label="Delete item" title={t('Delete')} onClick={() => void remove(item.id)}><Trash2 size={16} /></button></div>
              <details className="item-details"><summary>{t('Details and tags')}</summary><dl><dt>{t('First copied')}</dt><dd>{new Date(item.first_copied).toLocaleString(locale)}</dd><dt>{t('Copies')}</dt><dd>{item.copy_count}</dd>{item.kind !== 'image' && <><dt>{t('Size')}</dt><dd>{new TextEncoder().encode(item.text).length.toLocaleString()} {t('bytes')}</dd></>}</dl><ItemExtras item={item} refresh={refresh} /></details>
            </> : <div className="hint">{t('Select an item to see its contents.')}</div>}
          </section>
        </div>
      </>}
    </main>
    {notice && <div className="toast" role="status">{notice}</div>}
  </div>;
}
