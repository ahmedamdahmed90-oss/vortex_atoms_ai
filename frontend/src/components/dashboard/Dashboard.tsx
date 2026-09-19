import { useDashboard } from '../../hooks/useDashboard';
import { OverviewTab } from './OverviewTab';
import { ModelsTab } from './ModelsTab';
import { KnowledgeTab } from './KnowledgeTab';
import { ToolsTab } from './ToolsTab';
import { LogsTab } from './LogsTab';
import { SettingsTab } from './SettingsTab';
import { SecurityTab } from './SecurityTab';
import { PerformanceTab } from './PerformanceTab';
import { Tabs } from '../ui/Tabs';
import { Server, Cpu, Database, Wrench, FileText, Settings, Shield, Activity } from 'lucide-react';

const dashboardTabs = [
  { id: 'overview', label: 'نظرة عامة', icon: <Server className="h-4 w-4" /> },
  { id: 'models', label: 'النماذج', icon: <Cpu className="h-4 w-4" /> },
  { id: 'knowledge', label: 'قاعدة المعرفة', icon: <Database className="h-4 w-4" /> },
  { id: 'tools', label: 'الأدوات', icon: <Wrench className="h-4 w-4" /> },
  { id: 'logs', label: 'السجلات', icon: <FileText className="h-4 w-4" /> },
  { id: 'settings', label: 'الإعدادات', icon: <Settings className="h-4 w-4" /> },
  { id: 'security', label: 'الأمان', icon: <Shield className="h-4 w-4" /> },
  { id: 'performance', label: 'الأداء', icon: <Activity className="h-4 w-4" /> },
];

export function Dashboard() {
  const { activeTab, setActiveTab } = useDashboard();

  const renderTabContent = () => {
    switch (activeTab) {
      case 'overview': return <OverviewTab />;
      case 'models': return <ModelsTab />;
      case 'knowledge': return <KnowledgeTab />;
      case 'tools': return <ToolsTab />;
      case 'logs': return <LogsTab />;
      case 'settings': return <SettingsTab />;
      case 'security': return <SecurityTab />;
      case 'performance': return <PerformanceTab />;
      default: return <OverviewTab />;
    }
  };

  return (
    <div className="animate-in">
      <Tabs tabs={dashboardTabs} activeTab={activeTab} onChange={setActiveTab} variant="pills" className="mb-6" />
      {renderTabContent()}
    </div>
  );
}