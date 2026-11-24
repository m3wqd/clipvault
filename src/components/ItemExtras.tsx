import { Image as ImageIcon } from 'lucide-react';
import { useEffect, useState } from 'react';
import type { Item, Named } from '../types';
import { request } from '../services/ipc';
import { useTranslation } from '../i18n';

export function ImagePreview({ id, thumbnail = false }: { id: number; thumbnail?: boolean }) {
  const { t } = useTranslation();
  const [src, setSrc] = useState('');
  const [error, setError] = useState('');
  useEffect(() => {
    let alive = true;
    setSrc(''); setError('');
    void request<string>({ action: 'thumbnail', id }).then(s => { if (alive) setSrc(s); })
      .catch(e => { if (alive) setError(String(e)); });
    return () => { alive = false; };
  }, [id]);
  if (thumbnail) return src ? <img className="image-thumbnail" src={src} alt={t('Clipboard image')} loading="lazy" /> : <ImageIcon size={20} aria-label={t('Clipboard image')} />;
  return error ? <span role="status" className="error">{error}</span> : src
    ? <img className="image-preview" src={src} alt={t('Clipboard image')} loading="lazy" />
    : <span className="hint">{t('Loading image…')}</span>;
}

export function ItemExtras({ item, refresh }: { item: Item; refresh: () => void }) {
  const { t } = useTranslation();
  const [tags, setTags] = useState('');
  const [collections, setCollections] = useState<Named[]>([]);
  const [error, setError] = useState('');
  useEffect(() => {
    let alive = true;
    setTags(''); setError('');
    void request<string[]>({ action: 'tags', id: item.id }).then(rows => { if (alive) setTags(rows.join(', ')); })
      .catch(e => { if (alive) setError(String(e)); });
    void request<Named[]>({ action: 'collections' }).then(rows => { if (alive) setCollections(rows); })
      .catch(e => { if (alive) setError(String(e)); });
    return () => { alive = false; };
  }, [item.id]);
  return <div className="extras">
    {error && <p role="alert">{error}</p>}
    <label>{t('Tags, comma separated')}<input value={tags} onChange={e => setTags(e.target.value)} /></label>
    <button onClick={() => {
      void request({ action: 'set_tags', id: item.id, names: tags.split(',').map(s => s.trim()).filter(Boolean) })
        .then(refresh).catch(e => setError(String(e)));
    }}>{t('Save tags')}</button>
    <label>{t('Add to collection')}<select defaultValue="" onChange={e => {
      const select = e.currentTarget;
      const collection = Number(select.value);
      if (collection) void request({ action: 'assign', id: item.id, collection, value: true })
        .then(() => { select.value = ''; refresh(); }).catch(err => setError(String(err)));
    }}><option value="">{t('Choose…')}</option>{collections.map(c => <option value={c.id} key={c.id}>{c.name}</option>)}</select></label>
  </div>;
}
