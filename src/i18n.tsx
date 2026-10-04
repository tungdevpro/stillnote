import { createContext, ReactNode, useCallback, useContext, useEffect, useMemo, useState } from "react";

const en = {
  // sidebar
  allNotes: "All Notes",
  notebooks: "Notebooks",
  newNotebook: "New notebook",
  notebookName: "Notebook name",
  rename: "Rename",
  deleteNotebook: "Delete notebook",
  noNotebooks: "No notebooks yet",
  tags: "Tags",
  trash: "Trash",
  notebookFallback: "Notebook",
  localOnly: "Local only",
  signInToSync: "Sign in to sync",
  syncing: "Syncing…",
  syncedAt: (when: string) => `Synced ${when}`,
  notSynced: "Not synced yet",
  syncNow: "Sync now",

  // note list
  showSidebar: "Show sidebar (⌘\\)",
  hideSidebar: "Hide sidebar (⌘\\)",
  newNote: "New note (⌘N)",
  emptyTrash: "Empty",
  search: "Search (⌘F)",
  clearSearch: "Clear search",
  noteList: "Notes",
  results: (n: number) => (n === 1 ? "1 result" : `${n} results`),
  noteCount: (n: number) => (n === 1 ? "1 note" : `${n} notes`),
  untitled: "Untitled",
  noContent: "No content",
  noMatches: (q: string) => `No notes match “${q}”.`,
  trashIsEmpty: "The Trash is empty.",
  noNotesYet: "No notes yet.",
  writeFirstNote: "Write your first note",

  // editor
  notebook: "Notebook",
  noNotebook: "No notebook",
  showPreview: "Preview (⌘E)",
  showEditor: "Edit (⌘E)",
  pin: "Pin to top",
  unpin: "Unpin",
  moveToTrash: "Move to Trash",
  inTrashBanner: "This note is in the Trash.",
  restore: "Restore",
  deleteForever: "Delete Forever",
  titlePlaceholder: "Title",
  edited: (when: string) => `Edited ${when}`,
  created: (when: string) => `Created ${when}`,
  bodyPlaceholder: "Start writing… (Markdown supported)",
  previewEmpty: "Nothing here yet.",
  addTag: "Add tag",
  removeTag: (tag: string) => `Remove tag ${tag}`,
  emptyEditor: "Select a note, or press ⌘N to write a new one.",

  // confirmations
  cancel: "Cancel",
  deleteNotebookTitle: (name: string) => `Delete notebook “${name}”?`,
  deleteNotebookBody: (n: number) =>
    n === 0
      ? "This notebook is empty."
      : `${n === 1 ? "1 note" : `${n} notes`} in this notebook will be moved to the Trash.`,
  emptyTrashTitle: "Empty the Trash?",
  emptyTrashBody: (n: number) =>
    `${n === 1 ? "1 note" : `${n} notes`} will be permanently deleted on all devices. This can't be undone.`,
  deleteNoteTitle: "Delete this note forever?",
  deleteNoteBody: "The note will be deleted on all devices. This can't be undone.",
  signOutTitle: "Sign out?",
  signOutBody:
    "Notes you've synced stay safe in Supabase. Keep a copy on this Mac, or remove it if this is a shared computer?",
  signOutKeep: "Sign Out, Keep Notes",
  removeFromMac: "Remove from this Mac",
  wipeTitle: "Remove notes from this Mac?",
  wipeBody: "Your notes will be synced one last time, then removed from this Mac. The copy in Supabase is kept.",
  wipeConfirm: "Sync and Remove",
  wipeFailed: (msg: string) => `Couldn't remove notes: ${msg}`,

  // account & settings
  settings: "Settings",
  account: "Account",
  syncWithSupabase: "Sync with Supabase",
  signedInAs: "Signed in as",
  lastSync: "Last sync",
  signOut: "Sign Out",
  done: "Done",
  backToSignIn: "← Back to sign in",
  configureSupabase: "Configure Supabase",
  signInIntro: "Notes are always saved on this Mac. Sign in to back them up and sync across devices.",
  email: "Email",
  password: "Password",
  noAccount: "No account? Sign up",
  haveAccount: "Already have an account? Sign in",
  working: "Working…",
  signIn: "Sign In",
  signUp: "Sign Up",
  confirmSent: (email: string) => `We sent a confirmation email to ${email}. Confirm it, then sign in here.`,
  settingsIntro: "Find both values in the Supabase Dashboard → Project Settings → API.",
  projectUrl: "Project URL",
  anonKey: "Anon / publishable key",
  save: "Save",
  language: "Language",

  // dates
  yesterday: "Yesterday",

  // errors, keyed by the `code` the Rust side sends
  errors: {
    database: (d: string) => `Database error: ${d}`,
    network: (d: string) => `Network error: ${d}`,
    data: (d: string) => `Data error: ${d}`,
    notConfigured: () => "Supabase isn't set up yet. Add the Project URL and anon key in Settings.",
    notSignedIn: () => "You're not signed in.",
    notFound: (d: string) => (d === "notebook" ? "Notebook not found." : "Note not found."),
    invalid: (d: string) =>
      d === "badUrl" ? "The Supabase URL must start with https://" : "Notebook name can't be empty.",
    invalidCredentials: () => "Incorrect email or password.",
    emailNotConfirmed: () => "Your email isn't confirmed yet. Check your inbox.",
    userExists: () => "This email is already registered.",
    weakPassword: () => "That password is too weak.",
    sessionInvalid: (d: string) => `Your session is no longer valid (${d}).`,
    server: (d: string) => `Supabase error ${d}`,
  } as Record<string, (detail: string) => string>,
};

export type Strings = typeof en;

const vi: Strings = {
  allNotes: "Tất cả ghi chú",
  notebooks: "Sổ tay",
  newNotebook: "Tạo sổ tay",
  notebookName: "Tên sổ tay",
  rename: "Đổi tên",
  deleteNotebook: "Xóa sổ tay",
  noNotebooks: "Chưa có sổ tay nào",
  tags: "Thẻ",
  trash: "Thùng rác",
  notebookFallback: "Sổ tay",
  localOnly: "Chỉ lưu trên máy",
  signInToSync: "Đăng nhập để đồng bộ",
  syncing: "Đang đồng bộ…",
  syncedAt: (when) => `Đã đồng bộ ${when}`,
  notSynced: "Chưa đồng bộ",
  syncNow: "Đồng bộ ngay",

  showSidebar: "Hiện thanh bên (⌘\\)",
  hideSidebar: "Ẩn thanh bên (⌘\\)",
  newNote: "Ghi chú mới (⌘N)",
  emptyTrash: "Dọn sạch",
  search: "Tìm kiếm (⌘F)",
  clearSearch: "Xóa tìm kiếm",
  noteList: "Danh sách ghi chú",
  results: (n) => `${n} kết quả`,
  noteCount: (n) => `${n} ghi chú`,
  untitled: "Không có tiêu đề",
  noContent: "Chưa có nội dung",
  noMatches: (q) => `Không tìm thấy ghi chú nào khớp “${q}”.`,
  trashIsEmpty: "Thùng rác trống.",
  noNotesYet: "Chưa có ghi chú nào.",
  writeFirstNote: "Viết ghi chú đầu tiên",

  notebook: "Sổ tay",
  noNotebook: "Không có sổ tay",
  showPreview: "Xem trước (⌘E)",
  showEditor: "Soạn thảo (⌘E)",
  pin: "Ghim lên đầu",
  unpin: "Bỏ ghim",
  moveToTrash: "Chuyển vào thùng rác",
  inTrashBanner: "Ghi chú này đang ở thùng rác.",
  restore: "Khôi phục",
  deleteForever: "Xóa vĩnh viễn",
  titlePlaceholder: "Tiêu đề",
  edited: (when) => `Sửa ${when}`,
  created: (when) => `Tạo ${when}`,
  bodyPlaceholder: "Bắt đầu viết… (hỗ trợ Markdown)",
  previewEmpty: "Chưa có nội dung.",
  addTag: "Thêm thẻ",
  removeTag: (tag) => `Bỏ thẻ ${tag}`,
  emptyEditor: "Chọn một ghi chú hoặc nhấn ⌘N để viết ghi chú mới.",

  cancel: "Hủy",
  deleteNotebookTitle: (name) => `Xóa sổ tay “${name}”?`,
  deleteNotebookBody: (n) =>
    n === 0 ? "Sổ tay này đang trống." : `${n} ghi chú trong sổ tay này sẽ được chuyển vào thùng rác.`,
  emptyTrashTitle: "Dọn sạch thùng rác?",
  emptyTrashBody: (n) => `${n} ghi chú sẽ bị xóa vĩnh viễn trên mọi thiết bị. Không thể hoàn tác.`,
  deleteNoteTitle: "Xóa vĩnh viễn ghi chú này?",
  deleteNoteBody: "Ghi chú sẽ bị xóa trên mọi thiết bị. Không thể hoàn tác.",
  signOutTitle: "Đăng xuất?",
  signOutBody:
    "Ghi chú đã đồng bộ vẫn an toàn trên Supabase. Giữ lại bản sao trên máy này, hay xóa nếu đây là máy dùng chung?",
  signOutKeep: "Đăng xuất, giữ ghi chú",
  removeFromMac: "Xóa khỏi máy",
  wipeTitle: "Xóa ghi chú khỏi máy này?",
  wipeBody: "Ghi chú sẽ được đồng bộ lần cuối rồi xóa khỏi máy này. Bản trên Supabase vẫn được giữ nguyên.",
  wipeConfirm: "Đồng bộ và xóa",
  wipeFailed: (msg) => `Chưa xóa được: ${msg}`,

  settings: "Cài đặt",
  account: "Tài khoản",
  syncWithSupabase: "Đồng bộ với Supabase",
  signedInAs: "Đăng nhập với",
  lastSync: "Lần đồng bộ gần nhất",
  signOut: "Đăng xuất",
  done: "Xong",
  backToSignIn: "← Quay lại đăng nhập",
  configureSupabase: "Cấu hình Supabase",
  signInIntro: "Ghi chú luôn được lưu trên máy. Đăng nhập để sao lưu và đồng bộ giữa các thiết bị.",
  email: "Email",
  password: "Mật khẩu",
  noAccount: "Chưa có tài khoản? Đăng ký",
  haveAccount: "Đã có tài khoản? Đăng nhập",
  working: "Đang xử lý…",
  signIn: "Đăng nhập",
  signUp: "Đăng ký",
  confirmSent: (email) => `Đã gửi email xác nhận tới ${email}. Xác nhận xong hãy quay lại đăng nhập.`,
  settingsIntro: "Lấy hai giá trị này trong Supabase Dashboard → Project Settings → API.",
  projectUrl: "Project URL",
  anonKey: "Anon / publishable key",
  save: "Lưu",
  language: "Ngôn ngữ",

  yesterday: "Hôm qua",

  errors: {
    database: (d) => `Lỗi cơ sở dữ liệu: ${d}`,
    network: (d) => `Lỗi kết nối mạng: ${d}`,
    data: (d) => `Lỗi dữ liệu: ${d}`,
    notConfigured: () => "Chưa cấu hình Supabase. Hãy nhập Project URL và anon key trong Cài đặt.",
    notSignedIn: () => "Bạn chưa đăng nhập.",
    notFound: (d) => (d === "notebook" ? "Không tìm thấy sổ tay." : "Không tìm thấy ghi chú."),
    invalid: (d) =>
      d === "badUrl" ? "URL Supabase phải bắt đầu bằng https://" : "Tên sổ tay không được để trống.",
    invalidCredentials: () => "Email hoặc mật khẩu không đúng.",
    emailNotConfirmed: () => "Email chưa được xác nhận. Hãy mở hộp thư để xác nhận.",
    userExists: () => "Email này đã được đăng ký.",
    weakPassword: () => "Mật khẩu quá yếu.",
    sessionInvalid: (d) => `Phiên đăng nhập không còn hợp lệ (${d}).`,
    server: (d) => `Supabase báo lỗi ${d}`,
  },
};

export type Lang = "en" | "vi";
export const LANGUAGES: { id: Lang; label: string }[] = [
  { id: "en", label: "English" },
  { id: "vi", label: "Tiếng Việt" },
];
const DICTS: Record<Lang, Strings> = { en, vi };
const LOCALES: Record<Lang, string> = { en: "en-US", vi: "vi-VN" };
const STORAGE_KEY = "language";
export const DEFAULT_LANG: Lang = "en";

function savedLang(): Lang {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return v === "vi" || v === "en" ? v : DEFAULT_LANG;
  } catch {
    return DEFAULT_LANG;
  }
}

interface I18n {
  lang: Lang;
  locale: string;
  t: Strings;
  setLang: (lang: Lang) => void;
  /** Human-readable message for anything thrown by `invoke` or JS. */
  errorText: (e: unknown) => string;
}

const I18nContext = createContext<I18n | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(savedLang);

  const setLang = useCallback((next: Lang) => {
    setLangState(next);
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      /* storage unavailable */
    }
  }, []);

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  const value = useMemo<I18n>(() => {
    const t = DICTS[lang];
    const errorText = (e: unknown) => {
      if (e && typeof e === "object" && "code" in e) {
        const { code, detail } = e as { code: string; detail?: string };
        const fn = t.errors[code];
        return fn ? fn(detail ?? "") : detail || code;
      }
      return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
    };
    return { lang, locale: LOCALES[lang], t, setLang, errorText };
  }, [lang, setLang]);

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18n {
  const ctx = useContext(I18nContext);
  if (!ctx) throw new Error("useI18n must be used inside <I18nProvider>");
  return ctx;
}
