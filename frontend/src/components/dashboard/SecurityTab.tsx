import { useCallback, useEffect, useState } from 'react';
import { Card, CardHeader, CardContent } from '../ui/Card';
import { Button } from '../ui/Button';
import { Badge } from '../ui/Badge';
import { Spinner } from '../ui/Spinner';
import { Shield, RefreshCw, KeyRound, ScrollText } from 'lucide-react';
import { api, ApiError } from '../../services/api';
import type { AdminStatusResponse, AdminMetricsResponse } from '../../types';

export function SecurityTab() {
  const [status, setStatus] = useState<AdminStatusResponse | null>(null);
  const [metrics, setMetrics] = useState<AdminMetricsResponse | null>(null);
  const [audit, setAudit] = useState<string[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [rotating, setRotating] = useState(false);
  const [rotated, setRotated] = useState(false);
  const [purging, setPurging] = useState(false);
  const [purged, setPurged] = useState<number | null>(null);
  const [reloading, setReloading] = useState(false);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const [s, a, m] = await Promise.all([api.admin.status(), api.admin.audit(50), api.admin.metrics()]);
      setStatus(s);
      setAudit(a.entries);
      setMetrics(m);
    } catch (e) {
      setError(e instanceof ApiError && e.status === 403
        ? 'تحتاج صلاحية الإدارة لعرض هذه الصفحة (توكن admin)'
        : 'تعذّر الاتصال بالخادم');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh().catch(() => undefined);
  }, [refresh]);

  const rotate = async () => {
    if (!window.confirm('تدوير توكن الإدارة؟ سيُبطل التوكن الحالي فوراً.')) return;
    setRotating(true);
    try {
      await api.admin.rotate();
      setRotated(true);
      await refresh();
      setTimeout(() => setRotated(false), 5000);
    } catch {
      setError('فشل التدوير');
    } finally {
      setRotating(false);
    }
  };

  const purgeSessions = async () => {
    if (!window.confirm('حذف جلسات المحادثة القديمة من القرص؟ (حسب مدة الاحتفاظ)')) return;
    setPurging(true);
    try {
      const r = await api.admin.sessionsPurge();
      setPurged(r.removed);
      await refresh();
      setTimeout(() => setPurged(null), 8000);
    } catch {
      setError('فشل التنظيف');
    } finally {
      setPurging(false);
    }
  };

  const reloadConfig = async () => {
    setReloading(true);
    try {
      await api.admin.reload();
      await refresh();
    } catch {
      setError('فشل إعادة التحميل');
    } finally {
      setReloading(false);
    }
  };

  // The header renders synchronously (no network wait) so the tab is
  // instantly navigable and E2E-stable with or without a backend.
  const header = (
    <div><h2 className="text-xl font-semibold text-gray-900 dark:text-gray-100">الأمان</h2><p className="text-gray-500 dark:text-gray-400">المصادقة والصلاحيات والتدقيق</p></div>
  );

  if (loading) {
    return (
      <div className="space-y-6 max-w-3xl">
        {header}
        <div className="flex items-center gap-2 text-gray-500"><Spinner /> جارٍ التحميل…</div>
      </div>
    );
  }

  if (error || !status) {
    return (
      <div className="space-y-6 max-w-3xl">
        {header}
        <Card><CardContent><p className="text-red-600 dark:text-red-400">{error ?? 'لا بيانات'}</p><div className="mt-3"><Button variant="secondary" onClick={refresh}>إعادة المحاولة</Button></div></CardContent></Card>
      </div>
    );
  }

  return (
    <div className="space-y-6 max-w-3xl">
      {header}

      <Card>
        <CardHeader title="وضع الحماية" description="الحالة الفعلية للخادم المتصل" />
        <CardContent className="space-y-3">
          <div className="flex items-center gap-2">
            <Shield className="h-4 w-4" />
            {status.auth_enabled
              ? <Badge variant="success">المصادقة مفعّلة (user / admin)</Badge>
              : <Badge variant="warning">المصادقة معطّلة — ثقة محلية فقط</Badge>}
            <Badge variant={status.allow_custom_models ? 'warning' : 'success'}>
              {status.allow_custom_models ? 'نماذج مخصصة مسموحة' : 'النماذج من القائمة المعتمدة فقط'}
            </Badge>
            {status.read_only && <Badge variant="warning">وضع القراءة فقط</Badge>}
            {status.admin_loopback_only
              ? <Badge variant="success">الإدارة loopback فقط</Badge>
              : <Badge variant="warning">الإدارة مسموحة عن بُعد</Badge>}
          </div>
          {metrics && (
            <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-sm">
              <div><p className="text-gray-500">طلبات كلية</p><p className="font-mono">{metrics.requests_total}</p></div>
              <div><p className="text-gray-500">مرفوض 401</p><p className="font-mono">{metrics.denied_401}</p></div>
              <div><p className="text-gray-500">مرفوض 403</p><p className="font-mono">{metrics.denied_403}</p></div>
              <div><p className="text-gray-500">مرفوض 429</p><p className="font-mono">{metrics.denied_429}</p></div>
            </div>
          )}
          <div className="grid grid-cols-2 md:grid-cols-3 gap-3 text-sm">
            <div><p className="text-gray-500">سقف الـ prompt</p><p className="font-mono">{status.max_prompt_chars}</p></div>
            <div><p className="text-gray-500">سقف الـ batch</p><p className="font-mono">{status.max_batch_prompts}</p></div>
            <div><p className="text-gray-500">سقف الاستيراد</p><p className="font-mono">{status.max_import_chars}</p></div>
            <div><p className="text-gray-500">نطاق الحرارة</p><p className="font-mono">{status.temperature_range[0]}–{status.temperature_range[1]}</p></div>
            <div><p className="text-gray-500">حدّ المعدل / 10ث</p><p className="font-mono">{status.rate_limit_per_10s}</p></div>
            <div><p className="text-gray-500">مجلدات المعرفة</p><p className="font-mono text-xs">{status.knowledge_dirs.join(', ')}</p></div>
          </div>
          {status.allowed_origins.length > 0 && (
            <p className="text-sm text-gray-500">أصول CORS إضافية: <span className="font-mono text-xs">{status.allowed_origins.join(', ')}</span></p>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader title="توكن الإدارة" description="التدوير يُبطل القديم فوراً (قيم التوكن لا تُعرض أبداً)" />
        <CardContent className="space-y-3">
          <div className="flex flex-wrap items-center gap-3">
            <Button variant="secondary" onClick={rotate} disabled={rotating || status.read_only} leftIcon={<KeyRound className="h-4 w-4" />}>
              {rotating ? 'جارٍ التدوير…' : 'تدوير توكن الإدارة'}
            </Button>
            <Button variant="secondary" onClick={reloadConfig} disabled={reloading || status.read_only}>
              {reloading ? 'جارٍ التحميل…' : 'إعادة تحميل الإعدادات'}
            </Button>
            {rotated && <span className="text-green-600 dark:text-green-400 text-sm">تم التدوير وحُفظ تلقائياً</span>}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader title="خصوصية الجلسات" description={`حذف تلقائي بعد ${status.sessions_retention_days} يوماً (مشفرة بـ DPAPI على Windows)`} />
        <CardContent className="space-y-3">
          <div className="flex flex-wrap items-center gap-3">
            <Button variant="secondary" onClick={purgeSessions} disabled={purging || status.read_only}>
              {purging ? 'جارٍ التنظيف…' : 'تنظيف الجلسات القديمة الآن'}
            </Button>
            {purged !== null && <span className="text-green-600 dark:text-green-400 text-sm">حُذفت {purged} جلسة</span>}
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader title="سجل التدقيق" description="آخر الإجراءات ذات الامتيازات (تبديل نموذج / استيراد / تدوير)" />
        <CardContent>
          {audit.length === 0 ? (
            <p className="text-sm text-gray-500">لا إدخالات بعد</p>
          ) : (
            <div className="space-y-1 max-h-64 overflow-auto" dir="ltr">
              {audit.map((line, i) => (
                <pre key={i} className="text-xs font-mono bg-gray-50 dark:bg-gray-800 rounded px-2 py-1 overflow-x-auto">{line}</pre>
              ))}
            </div>
          )}
          <div className="mt-3"><Button variant="secondary" onClick={refresh} leftIcon={<ScrollText className="h-4 w-4" />}>تحديث <RefreshCw className="h-3 w-3" /></Button></div>
        </CardContent>
      </Card>
    </div>
  );
}
