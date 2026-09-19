// Copyright (c) 2026 Ahmad Mansour — Vortex Atoms AI
import { useEffect, useState } from 'react';
import { useDashboard } from '../../hooks/useDashboard';
import { Card, CardHeader, CardContent } from '../ui/Card';
import { Badge } from '../ui/Badge';
import { Spinner } from '../ui/Spinner';
import { api } from '../../services/api';
import type { BenchResponse } from '../../types';
import { Activity, Zap, Cpu, Layers } from 'lucide-react';

export function PerformanceTab() {
  const { health } = useDashboard();
  const [bench, setBench] = useState<BenchResponse | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchBench = async () => {
    setLoading(true);
    setError(null);
    try {
      const data = await api.bench();
      setBench(data);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchBench();
  }, []);

  const perf = health?.perf;

  return (
    <div className="space-y-6">
      {/* Perf summary from /v1/health */}
      <Card>
        <CardHeader title="الأداء — ملخص سريع" description="بيانات مباشرة من /v1/health.perf — نفس المدار الذي يغذي /v1/bench" />
        <CardContent>
          {perf ? (
            <div className="grid grid-cols-2 lg:grid-cols-4 gap-4">
              <div className="p-4 rounded-xl bg-gray-50 dark:bg-gray-800">
                <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400"><Cpu className="h-4 w-4" /> SKU</div>
                <div className="mt-1 font-mono font-bold">{perf.sku}</div>
                <div className="text-xs text-gray-500">{perf.compiled_features.join(' + ') || 'sse2'}</div>
              </div>
              <div className="p-4 rounded-xl bg-gray-50 dark:bg-gray-800">
                <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400"><Layers className="h-4 w-4" /> Tier</div>
                <div className="mt-1 font-bold">{perf.tier} <span className="font-normal text-gray-500">({perf.tier_model})</span></div>
                <div className="text-xs text-gray-500">ctx {perf.tier_max_context} · KV f16</div>
              </div>
              <div className="p-4 rounded-xl bg-gray-50 dark:bg-gray-800">
                <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400"><Activity className="h-4 w-4" /> Threads</div>
                <div className="mt-1 font-mono">infer {perf.infer_threads} · async {perf.async_workers}</div>
                <div className="text-xs text-gray-500">prefault {perf.prefault_enabled ? 'on' : 'off'}</div>
              </div>
              <div className="p-4 rounded-xl bg-gray-50 dark:bg-gray-800">
                <div className="flex items-center gap-2 text-sm text-gray-500 dark:text-gray-400"><Zap className="h-4 w-4" /> Fastpath</div>
                <div className="mt-1 font-mono font-bold">{perf.fastpath_avg_ms.toFixed(2)} ms</div>
                <div className="text-xs text-gray-500">semantic cache &lt;1 ms</div>
              </div>
            </div>
          ) : (
            <div className="flex items-center gap-2 text-sm text-gray-500"><Spinner size="sm" /> جارٍ تحميل /v1/health …</div>
          )}
          <div className="mt-4 flex gap-2">
            <Badge variant={perf?.fastpath_avg_ms !== undefined && perf.fastpath_avg_ms < 50 ? 'success' : 'warning'}>
              fastpath {perf ? `${perf.fastpath_avg_ms.toFixed(1)}ms` : '—'} {perf && perf.fastpath_avg_ms < 50 ? '✓ <50ms' : ''}
            </Badge>
            <Badge variant="default">{health ? `up ${health.uptime_seconds}s` : '—'}</Badge>
          </div>
        </CardContent>
      </Card>

      {/* Bench matrix from /v1/bench */}
      <Card>
        <CardHeader title="مصفوفة الأداء — /v1/bench" description="SKUs × tok/s — تتوسع في PERF-02 §2/§7 إلى backends×quants×spec%" />
        <CardContent>
          <div className="flex items-center justify-between mb-4">
            <button
              onClick={fetchBench}
              disabled={loading}
              className="px-4 py-2 rounded-lg bg-primary-600 text-white text-sm hover:bg-primary-700 disabled:opacity-50"
            >
              {loading ? 'جارٍ القياس…' : 'إعادة القياس'}
            </button>
            {bench && <span className="text-xs text-gray-500">sku_current={bench.sku_current} · tier={bench.tier} · {bench.generated_at}</span>}
          </div>
          {error && <div className="p-3 rounded-lg bg-red-50 dark:bg-red-900/20 text-sm text-red-700 dark:text-red-400">{error}</div>}
          {bench ? (
            <div className="overflow-x-auto">
              <table className="w-full text-sm">
                <thead>
                  <tr className="text-left text-gray-500 dark:text-gray-400 border-b">
                    <th className="py-2 px-3">SKU</th>
                    <th className="py-2 px-3">TTFT</th>
                    <th className="py-2 px-3">tok/s</th>
                    <th className="py-2 px-3">RSS</th>
                    <th className="py-2 px-3">fastpath</th>
                    <th className="py-2 px-3">cache-hit</th>
                    <th className="py-2 px-3">compat</th>
                    <th className="py-2 px-3">notes</th>
                  </tr>
                </thead>
                <tbody>
                  {bench?.entries?.map((e) => (
                    <tr key={e.sku} className="border-b border-gray-100 dark:border-gray-800">
                      <td className="py-2 px-3 font-mono font-bold">{e.sku}</td>
                      <td className="py-2 px-3">{e.ttft_ms ?? '—'}</td>
                      <td className="py-2 px-3">{e.tok_per_s ?? '—'}</td>
                      <td className="py-2 px-3">{e.peak_rss_mb ?? '—'}</td>
                      <td className="py-2 px-3">{e.fastpath_ms.toFixed(2)} ms</td>
                      <td className="py-2 px-3">{e.cache_hit_ms.toFixed(2)} ms</td>
                      <td className="py-2 px-3">{e.compatible ? '✓' : '✗'}</td>
                      <td className="py-2 px-3 text-xs text-gray-500">{e.notes}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            !error && <div className="flex items-center gap-2 text-sm text-gray-500"><Spinner size="sm" /> بانتظار /v1/bench …</div>
          )}
          <p className="mt-4 text-xs text-gray-500 dark:text-gray-400">
            PERF-01 baseline مجمّد في <code>docs/PERF_REPORT.md</code>. الأرقام الحية هنا للفحص السريع؛ §7 يضيف ring-buffer + chart + flamegraph.
          </p>
        </CardContent>
      </Card>
    </div>
  );
}
