import { useState } from "react";
import { Hash, X } from "lucide-react";
import { useI18n } from "../i18n";

interface TagEditorProps {
  tags: string[];
  readOnly: boolean;
  onChange: (tags: string[]) => void;
}

export function TagEditor({ tags, readOnly, onChange }: TagEditorProps) {
  const [draft, setDraft] = useState("");
  const { t } = useI18n();

  const commit = (text = draft) => {
    const parts = text
      .split(",")
      .map((s) => s.trim().replace(/^#/, ""))
      .filter(Boolean);
    setDraft("");
    if (parts.length) onChange([...tags, ...parts]);
  };

  if (readOnly && tags.length === 0) return null;

  return (
    <div className="tag-editor">
      <Hash size={13} className="tag-editor-icon" />
      {tags.map((tag) => (
        <span key={tag} className="tag-chip">
          {tag}
          {!readOnly && (
            <button
              className="tag-remove"
              title={t.removeTag(tag)}
              onClick={() => onChange(tags.filter((x) => x !== tag))}
            >
              <X size={11} />
            </button>
          )}
        </span>
      ))}
      {!readOnly && (
        <input
          className="tag-input"
          value={draft}
          placeholder={tags.length ? "" : t.addTag}
          onChange={(e) => {
            const v = e.target.value;
            if (v.includes(",")) commit(v);
            else setDraft(v);
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              commit();
            } else if (e.key === "Backspace" && !draft && tags.length) {
              onChange(tags.slice(0, -1));
            }
          }}
          onBlur={() => commit()}
        />
      )}
    </div>
  );
}
