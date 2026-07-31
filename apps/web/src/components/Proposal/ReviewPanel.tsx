import { useState } from 'react';
import { useTranslate } from '../../i18n/useI18n';
import { generateBriefing } from '../../proposal/briefing';
import { generateScaffolding } from '../../proposal/scaffolding';
import { serializeChangeSet } from '../../proposal/serialize';
import type { ChangeSet } from '../../proposal/types';
import './ReviewPanel.css';

interface ReviewPanelProps {
  changeSet: ChangeSet | null;
  note: string;
  onNoteChange: (note: string) => void;
  onClose: () => void;
  locale: 'en' | 'pt-BR';
}

type TabType = 'briefing' | 'scaffolding' | 'json';

export function ReviewPanel({ changeSet, note, onNoteChange, onClose, locale }: ReviewPanelProps) {
  const t = useTranslate();
  const [activeTab, setActiveTab] = useState<TabType>('briefing');
  const [copied, setCopied] = useState<string | null>(null);

  if (!changeSet) return null;

  const briefingMarkdown = generateBriefing(changeSet, locale, note);
  const scaffoldingCode = generateScaffolding(changeSet);
  const jsonContent = serializeChangeSet(changeSet);

  const handleCopy = (content: string, type: string) => {
    navigator.clipboard.writeText(content).then(() => {
      setCopied(type);
      setTimeout(() => setCopied(null), 2000);
    });
  };

  return (
    <div className="review-modal-overlay" onClick={onClose}>
      <div className="review-panel" onClick={(e) => e.stopPropagation()}>
        <div className="review-panel-header">
          <div className="review-panel-title">
            <span>{t('proposal.reviewTitle')}</span>
            <span className="change-count-badge">{changeSet.totalChanges}</span>
          </div>
          <button type="button" className="icon-button" onClick={onClose}>
            ✕
          </button>
        </div>

        <div className="review-tabs">
          <button
            type="button"
            className={`tab-button ${activeTab === 'briefing' ? 'active' : ''}`}
            onClick={() => setActiveTab('briefing')}
          >
            {t('proposal.tabBriefing')}
          </button>
          <button
            type="button"
            className={`tab-button ${activeTab === 'scaffolding' ? 'active' : ''}`}
            onClick={() => setActiveTab('scaffolding')}
          >
            {t('proposal.tabScaffolding')}
          </button>
          <button
            type="button"
            className={`tab-button ${activeTab === 'json' ? 'active' : ''}`}
            onClick={() => setActiveTab('json')}
          >
            {t('proposal.tabJson')}
          </button>
        </div>

        <div className="review-content">
          {activeTab === 'briefing' && (
            <>
              <div className="note-input-container">
                <label className="text-dim">{t('proposal.authorNoteLabel')}</label>
                <textarea
                  className="note-textarea"
                  value={note}
                  onChange={(e) => onNoteChange(e.target.value)}
                  placeholder={t('proposal.notePlaceholder')}
                  rows={2}
                />
              </div>
              <pre className="code-preview">{briefingMarkdown}</pre>
            </>
          )}

          {activeTab === 'scaffolding' && <pre className="code-preview">{scaffoldingCode}</pre>}

          {activeTab === 'json' && <pre className="code-preview">{jsonContent}</pre>}
        </div>

        <div className="review-panel-footer">
          <div>
            {copied ? <span className="copy-feedback">✓ {copied} {t('proposal.copied')}</span> : null}
          </div>
          <div className="form-actions">
            {activeTab === 'briefing' && (
              <button
                type="button"
                className="button-primary"
                onClick={() => handleCopy(briefingMarkdown, 'Briefing')}
              >
                📋 {t('proposal.copyBriefing')}
              </button>
            )}
            {activeTab === 'scaffolding' && (
              <button
                type="button"
                className="button-primary"
                onClick={() => handleCopy(scaffoldingCode, 'Rust Scaffolding')}
              >
                📋 {t('proposal.copyScaffolding')}
              </button>
            )}
            {activeTab === 'json' && (
              <button
                type="button"
                className="button-primary"
                onClick={() => handleCopy(jsonContent, 'JSON')}
              >
                📋 {t('proposal.copyJson')}
              </button>
            )}
            <button type="button" className="button-secondary" onClick={onClose}>
              {t('proposal.close')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
