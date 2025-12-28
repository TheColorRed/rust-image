import { DocumentProperties } from '@/components/panels/properties/document';
import { RasterProperties } from '@/components/panels/properties/panel';
import { titleCase } from '@/lib/strings';
import { LayerMetadata } from '@alakazam/abra';
import { createContext, useContext, useEffect, useState } from 'react';

export const PropertiesContext = createContext({
  activeLayer: null as LayerMetadata | null,
});

function AdjustmentLayerProperties() {
  const { activeLayer } = useContext(PropertiesContext);

  return <h3 className="text-base font-medium">{titleCase(activeLayer?.adjustmentType ?? '')}</h3>;
}

export function PropertiesPanel() {
  const [activeLayer, setActiveLayer] = useState<LayerMetadata | null>(null);

  useEffect(() => {
    const unsubscribe = window.alakazam.projects.onProjectChanged(project => {
      const layer = project.activeLayers.length > 0 ? project.activeLayers[0] : null;
      setActiveLayer(layer);
    });
    return () => {
      unsubscribe();
    };
  }, []);

  return (
    <PropertiesContext.Provider value={{ activeLayer }}>
      <div className="min-h-50 space-y-4">
        <h2 className="text-lg font-medium">Properties</h2>
        {activeLayer && activeLayer.adjustmentType ? (
          <AdjustmentLayerProperties />
        ) : activeLayer ? (
          <RasterProperties />
        ) : (
          <DocumentProperties />
        )}
      </div>
    </PropertiesContext.Provider>
  );
}
PropertiesPanel.displayName = 'PropertiesPanel';
