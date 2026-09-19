import { clsx } from 'clsx';
import { Server } from 'lucide-react';
import { useDashboard } from '../../hooks/useDashboard';
import { Card, CardHeader, CardContent } from '../ui/Card';

import { Spinner } from '../ui/Spinner';
import { formatUptime } from '../../utils/helpers';

export function OverviewTab() {
  const { health, loading, fetchHealth } = useDashboard();

  if (loading && !health) {
    return <div className="flex items-center justify-center h-64"><Spinner size="lg" /></div>;
  }

  if (!health) {
    return (
      <div className="text-center py-12">
        <svg className="h-12 w-12 text-gray-400 mx-auto mb-4" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.462 0z" /></svg>
        <p className="text-gray-500 dark:text-gray-400 mt-2">لا توجد بيانات حالة الخادم</p>
        <button onClick={fetchHealth} className="mt-4 text-primary-600 dark:text-primary-400 hover:underline">تحديث</button>
      </div>
    );
  }

  const stats = [
    { label: 'وقت التشغيل', value: formatUptime(health.uptime_seconds), icon: <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" /></svg>, color: 'text-blue-600' },
    { label: 'النموذج', value: health.architecture, icon: <Server className="h-5 w-5" />, color: 'text-purple-600' },
    { label: 'الجهاز', value: health.device, icon: <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" /></svg>, color: 'text-green-600' },
    { label: 'قطع المعرفة', value: health.knowledge_chunks.toString(), icon: <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M4 7v10c0 2.2 1.8 4 4 4h12c2.2 0 4-1.8 4-4V7" /></svg>, color: 'text-orange-600' },
  ];

  return (
    <div className="space-y-6">
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {stats.map((stat) => (
          <Card key={stat.label} hover>
            <CardContent className="flex items-center gap-4">
              <div className={clsx('p-3 rounded-xl bg-gray-100 dark:bg-gray-800', stat.color)}>{stat.icon}</div>
              <div><p className="text-sm text-gray-500 dark:text-gray-400">{stat.label}</p><p className="text-xl font-bold text-gray-900 dark:text-gray-100">{stat.value}</p></div>
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
        <Card>
          <CardHeader title="حالة الاتصال" description="معلومات حالة الخادم الحالية" />
          <CardContent>
            <div className="space-y-4">
              <div className="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
                <div className="flex items-center gap-3"><div className="p-2 rounded-lg bg-green-100 dark:bg-green-900/30"><svg className="h-5 w-5 text-green-600 dark:text-green-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 13l4 4L19 7" /></svg></div><div><p className="font-medium text-gray-900 dark:text-gray-100">حالة الخادم</p><p className="text-sm text-gray-500 dark:text-gray-400">{health.status}</p></div></div>
                <span className="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400">متصل</span>
              </div>
              <div className="grid grid-cols-2 gap-4 text-sm">
                <div className="p-3 rounded-lg bg-gray-50 dark:bg-gray-800"><p className="text-gray-500 dark:text-gray-400">المعمارية</p><p className="font-medium text-gray-900 dark:text-gray-100">{health.architecture}</p></div>
                <div className="p-3 rounded-lg bg-gray-50 dark:bg-gray-800"><p className="text-gray-500 dark:text-gray-400">SIMD</p><p className="font-medium text-gray-900 dark:text-gray-100">{health.simd}</p></div>
                <div className="p-3 rounded-lg bg-gray-50 dark:bg-gray-800"><p className="text-gray-500 dark:text-gray-400">سجل المحادثات</p><p className="font-medium text-gray-900 dark:text-gray-100">{health.history_length}</p></div>
                <div className="p-3 rounded-lg bg-gray-50 dark:bg-gray-800"><p className="text-gray-500 dark:text-gray-400">قطع المعرفة</p><p className="font-medium text-gray-900 dark:text-gray-100">{health.knowledge_chunks}</p></div>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader title="معلومات الجهاز" description="تفاصيل بيئة التشغيل" />
          <CardContent>
            <div className="space-y-3">
              <div className="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800"><div className="flex items-center gap-3"><div className="p-2 rounded-lg bg-purple-100 dark:bg-purple-900/30"><svg className="h-5 w-5 text-purple-600 dark:text-purple-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" /></svg></div><div><p className="text-sm text-gray-500 dark:text-gray-400">الجهاز</p><p className="font-medium text-gray-900 dark:text-gray-100">{health.device}</p></div></div></div>
              <div className="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800"><div className="flex items-center gap-3"><div className="p-2 rounded-lg bg-blue-100 dark:bg-blue-900/30"><svg className="h-5 w-5 text-blue-600 dark:text-blue-400" fill="none" viewBox="0 0 24 24" stroke="currentColor"><path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" /></svg></div><div><p className="text-sm text-gray-500 dark:text-gray-400">دعم SIMD</p><p className="font-medium text-gray-900 dark:text-gray-100 text-sm">{health.simd}</p></div></div></div>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}