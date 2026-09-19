import { useCallback, useEffect, useState } from 'react';
import { useDashboardStore } from '../stores/dashboardStore';
import { api } from '../services/api';
import type { ModelInfo } from '../types';

export function useDashboard() {
  const {
    activeTab,
    setActiveTab,
    health,
    setHealth,
    models,
    setModels,
    currentModel,
    setCurrentModel,
    knowledgeChunks,
    searchResults,
    setSearchResults,
    tools,
    setTools,
    addLog,
    logs,
    logFilter,
    setLogFilter,
    clearLogs,
    sidebarOpen,
    setSidebarOpen,
  } = useDashboardStore();

  const [loading, setLoading] = useState(false);

  const fetchHealth = useCallback(async () => {
    try {
      const data = await api.health();
      setHealth(data);
      addLog(`[${new Date().toLocaleTimeString()}] Health check: ${data.status}`);
    } catch (error) {
      addLog(`[${new Date().toLocaleTimeString()}] Health check failed: ${error}`);
    }
  }, [setHealth, addLog]);

  const fetchModels = useCallback(async () => {
    try {
      const [modelInfo] = await Promise.all([api.models()]);
      const info = modelInfo as unknown as ModelInfo;
      setModels([{ name: info.architecture, architecture: info.architecture, max_seq_len: info.max_seq_len, max_generation_tokens: info.max_generation_tokens }]);
      if (info.architecture) {
        setCurrentModel(info.architecture);
      }
    } catch (error) {
      addLog(`[${new Date().toLocaleTimeString()}] Models fetch failed: ${error}`);
    }
  }, [setModels, setCurrentModel, addLog]);

  const fetchTools = useCallback(async () => {
    try {
      const data = await api.tools.list();
      setTools(data.tools);
    } catch (error) {
      addLog(`[${new Date().toLocaleTimeString()}] Tools fetch failed: ${error}`);
    }
  }, [setTools, addLog]);

  const swapModel = useCallback(async (model: string) => {
    setLoading(true);
    try {
      await api.swapModel({ model });
      setCurrentModel(model);
      addLog(`[${new Date().toLocaleTimeString()}] Model swapped to: ${model}`);
      await fetchModels();
    } catch (error) {
      addLog(`[${new Date().toLocaleTimeString()}] Model swap failed: ${error}`);
    } finally {
      setLoading(false);
    }
  }, [setCurrentModel, fetchModels, addLog, setLoading]);

  const searchKnowledge = useCallback(async (query: string) => {
    try {
      const data = await api.knowledge.search({ query, limit: 10 });
      setSearchResults(data.results);
      addLog(`[${new Date().toLocaleTimeString()}] Knowledge search: "${query}" - ${data.results.length} results`);
    } catch (error) {
      addLog(`[${new Date().toLocaleTimeString()}] Knowledge search failed: ${error}`);
    }
  }, [setSearchResults, addLog]);

  const importKnowledge = useCallback(async (text: string) => {
    try {
      const data = await api.knowledge.import({ text });
      addLog(`[${new Date().toLocaleTimeString()}] Knowledge imported: ${data.chunks_imported} chunks`);
      await fetchHealth();
    } catch (error) {
      addLog(`[${new Date().toLocaleTimeString()}] Knowledge import failed: ${error}`);
    }
  }, [fetchHealth, addLog]);

  const executeTool = useCallback(async (name: string, arguments_: Record<string, unknown>) => {
    try {
      const start = Date.now();
      const data = await api.tools.execute({ name, arguments: arguments_ });
      const duration = Date.now() - start;
      addLog(`[${new Date().toLocaleTimeString()}] Tool ${name} executed in ${duration}ms`);
      return data;
    } catch (error) {
      addLog(`[${new Date().toLocaleTimeString()}] Tool ${name} failed: ${error}`);
      throw error;
    }
  }, [addLog]);

  const refreshAll = useCallback(async () => {
    setLoading(true);
    await Promise.all([fetchHealth(), fetchModels(), fetchTools()]);
    setLoading(false);
  }, [fetchHealth, fetchModels, fetchTools, setLoading]);

  useEffect(() => {
    fetchHealth();
    fetchModels();
    fetchTools();
  }, [fetchHealth, fetchModels, fetchTools]);

  return {
    activeTab,
    setActiveTab,
    health,
    models,
    currentModel,
    knowledgeChunks,
    searchResults,
    tools,
    logs,
    logFilter,
    loading,
    fetchHealth,
    fetchModels,
    fetchTools,
    swapModel,
    searchKnowledge,
    importKnowledge,
    executeTool,
    refreshAll,
    setLogFilter,
    clearLogs,
    sidebarOpen,
    setSidebarOpen,
  };
}