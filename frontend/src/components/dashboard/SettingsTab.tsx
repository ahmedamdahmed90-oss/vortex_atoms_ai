import { useSettingsStore } from '../../stores/settingsStore';
import { Card, CardHeader, CardContent } from '../ui/Card';
import { Button } from '../ui/Button';
import { Input } from '../ui/Input';
import { Select } from '../ui/Select';
import { useState, useEffect } from 'react';
import { RefreshCw, Download, KeyRound } from 'lucide-react';
import { setTokens, resetAuth, api } from '../../services/api';
import { wsService } from '../../services/ws';

export function SettingsTab() {
  const { language, setLanguage, theme, setTheme, apiUrl, setApiUrl, wsUrl, setWsUrl } = useSettingsStore();
  const [testResult, setTestResult] = useState<string | null>(null);
  const [apiTokenInput, setApiTokenInput] = useState('');
  const [adminTokenInput, setAdminTokenInput] = useState('');
  const [tokenSaved, setTokenSaved] = useState(false);
  const [coalesceInput, setCoalesceInput] = useState('50');
  const [tierPolicy, setTierPolicy] = useState<string | null>(null);
  const [knobsMsg, setKnobsMsg] = useState<{ kind: 'ok' | 'err'; text: string } | null>(null);

  // PERF-02 §6.4: progressive-UX knobs. GET/POST /v1/admin/performance are
  // admin-gated; without an admin token the card shows a hint, not an error.
  useEffect(() => {
    let cancelled = false;
    api.admin.performance.get()
      .then((k) => {
        if (cancelled) return;
        setCoalesceInput(String(k.ws_coalesce_ms));
        setTierPolicy(k.tier_policy);
      })
      .catch(() => {
        if (cancelled) return;
        setTierPolicy(null);
      });
    return () => { cancelled = true; };
  }, []);

  const saveKnobs = () => {
    const v = Math.min(5000, Math.max(0, Math.round(Number(coalesceInput) || 0)));
    setCoalesceInput(String(v));
    api.admin.performance.update({ ws_coalesce_ms: v })
      .then((k) => {
        setCoalesceInput(String(k.ws_coalesce_ms));
        setTierPolicy(k.tier_policy);
        setKnobsMsg({ kind: 'ok', text: `تم الحفظ — يُطبَّق على المقابس الجديدة (النافذة ${k.ws_coalesce_ms}ms)` });
        setTimeout(() => setKnobsMsg(null), 4000);
      })
      .catch((e) => {
        setKnobsMsg({ kind: 'err', text: e instanceof Error ? e.message : 'فشل الحفظ (يتطلب توكن الإدارة)' });
        setTimeout(() => setKnobsMsg(null), 4000);
      });
  };

  const testConnection = async () => {
    try { const response = await fetch(`${apiUrl}/health`, { method: 'GET' }); if (response.ok) { setTestResult('success'); } else { setTestResult('error'); } } catch { setTestResult('error'); } setTimeout(() => setTestResult(null), 3000);
  };

  // Remote deployments (LAN/TLS): the loopback bootstrap refuses, so the
  // operator pastes tokens out-of-band. Memory-only — never persisted.
  const saveTokens = () => {
    setTokens(apiTokenInput.trim() || null, adminTokenInput.trim() || null);
    setApiTokenInput('');
    setAdminTokenInput('');
    setTokenSaved(true);
    // Reconnect the socket so it picks up the new token immediately.
    try { wsService.disconnect(); } catch { /* ignore */ }
    wsService.connect().catch(() => undefined);
    setTimeout(() => setTokenSaved(false), 4000);
  };

  const forgetTokens = () => {
    resetAuth();
    setApiTokenInput('');
    setAdminTokenInput('');
    try { wsService.disconnect(); } catch { /* ignore */ }
    wsService.connect().catch(() => undefined);
  };

  const exportSettings = () => {
    const state = useSettingsStore.getState();
    const payload = { language: state.language, theme: state.theme, apiUrl: state.apiUrl, wsUrl: state.wsUrl, animationsEnabled: state.animationsEnabled, compactMode: state.compactMode };
    const blob = new Blob([JSON.stringify(payload, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'vortex-settings.json';
    a.click();
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6 max-w-3xl">
      <div><h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">الإعدادات</h2><p className="text-gray-500 dark:text-gray-400">تخصيص التطبيق وتفضيلاتك</p></div>

      <Card><CardHeader title="عام" description="إعدادات اللغة والمظهر" /><CardContent className="space-y-6"><div><label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">اللغة</label><Select value={language} onChange={(e) => setLanguage(e.target.value as 'ar' | 'en')} options={[{ value: 'ar', label: 'العربية' },{ value: 'en', label: 'English' }]} /></div><div><label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">المظهر</label><Select value={theme} onChange={(e) => setTheme(e.target.value as 'light' | 'dark' | 'system')} options={[{ value: 'light', label: 'فاتح' },{ value: 'dark', label: 'داكن' },{ value: 'system', label: 'النظام' }]} /></div></CardContent></Card>

      <Card><CardHeader title="API" description="إعدادات الاتصال بالخادم" /><CardContent className="space-y-4"><Input label="عنوان API" value={apiUrl} onChange={(e) => setApiUrl(e.target.value)} placeholder="http://localhost:8080/v1" /><Input label="عنوان WebSocket" value={wsUrl} onChange={(e) => setWsUrl(e.target.value)} placeholder="ws://localhost:8080/ws" /><div className="flex items-center gap-3"><Button variant="secondary" onClick={testConnection} leftIcon={<svg className="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" /></svg>}>اختبار الاتصال</Button>{testResult === 'success' && <span className="text-green-600 dark:text-green-400 text-sm">نجح الاتصال</span>}{testResult === 'error' && <span className="text-red-600 dark:text-red-400 text-sm">فشل الاتصال</span>}</div></CardContent></Card>

      <Card><CardHeader title="الأداء" description="نافذة دمج توكنات WebSocket (تُطبَّق على المقابس الجديدة). تتطلب صلاحية الإدارة." /><CardContent className="space-y-4"><Input label="نافذة الدمج ws_coalesce_ms (ms، من 0 إلى 5000 — 0 = إطار لكل توكن)" type="number" value={coalesceInput} onChange={(e) => setCoalesceInput(e.target.value)} placeholder="50" /><div className="text-sm text-gray-500 dark:text-gray-400">سياسة الدرجات tier_policy: <span className="font-mono">{tierPolicy ?? '— (تتطلب توكن الإدارة للعرض)'}</span></div><div className="flex items-center gap-3"><Button variant="secondary" onClick={saveKnobs}>حفظ الأداء</Button>{knobsMsg?.kind === 'ok' && <span className="text-green-600 dark:text-green-400 text-sm">{knobsMsg.text}</span>}{knobsMsg?.kind === 'err' && <span className="text-red-600 dark:text-red-400 text-sm">{knobsMsg.text}</span>}</div></CardContent></Card>

      <Card><CardHeader title="توكنات الوصول" description="للخوادم البعيدة فقط — محلياً تُسحب تلقائياً. تُحفظ في الذاكرة فقط ولا تُخزّن أبداً." /><CardContent className="space-y-4"><Input label="توكن المستخدم (vxt_…)" type="password" value={apiTokenInput} onChange={(e) => setApiTokenInput(e.target.value)} placeholder="الصق من مضيف الخادم" autoComplete="off" /><Input label="توكن الإدارة (vxa_…، اختياري)" type="password" value={adminTokenInput} onChange={(e) => setAdminTokenInput(e.target.value)} placeholder="للإدارة عن بُعد" autoComplete="off" /><div className="flex items-center gap-3"><Button variant="secondary" onClick={saveTokens} leftIcon={<KeyRound className="h-4 w-4" />}>حفظ وتفعيل</Button><Button variant="secondary" onClick={forgetTokens}>نسيان التوكنات</Button>{tokenSaved && <span className="text-green-600 dark:text-green-400 text-sm">تم التفعيل (جلسة الذاكرة فقط)</span>}</div></CardContent></Card>

      <Card><CardHeader title="الإشعارات" description="تفضيلات الإشعارات" /><CardContent className="space-y-4"><label className="flex items-center justify-between"><div><p className="font-medium text-gray-900 dark:text-gray-100">إشعارات الاتصال</p><p className="text-sm text-gray-500 dark:text-gray-400">تلقي إشعارات عند الاتصال/انقطاع الاتصال</p></div><input type="checkbox" defaultChecked className="w-5 h-5 rounded border-gray-300 text-primary-600 focus:ring-primary-500" /></label><label className="flex items-center justify-between"><div><p className="font-medium text-gray-900 dark:text-gray-100">إشعارات الأخطاء</p><p className="text-sm text-gray-500 dark:text-gray-400">تنبيه عند حدوث أخطاء في API</p></div><input type="checkbox" defaultChecked className="w-5 h-5 rounded border-gray-300 text-primary-600 focus:ring-primary-500" /></label></CardContent></Card>

      <Card><CardHeader title="الخصوصية والأمان" /><CardContent className="space-y-4"><div className="grid grid-cols-1 md:grid-cols-2 gap-4"><Button variant="secondary" onClick={() => useSettingsStore.getState().resetToDefaults()} leftIcon={<RefreshCw className="h-4 w-4" />}>استعادة الإعدادات الافتراضية</Button><Button variant="secondary" onClick={exportSettings} leftIcon={<Download className="h-4 w-4" />}>تصدير الإعدادات</Button></div></CardContent></Card>
    </div>
  );
}