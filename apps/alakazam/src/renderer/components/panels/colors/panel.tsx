import { ColorPicker } from '@/components/panels/colors/color-picker';
import { Swatches } from '@/components/panels/colors/swatches';
import { TabContent, TabItem, TabLabel, Tabs } from '@/ui/tabs';

export function ColorsPanel() {
  return (
    <Tabs borderX={false} borderB={false}>
      <TabItem>
        <TabLabel>Colors</TabLabel>
        <TabContent className="p-4">
          <ColorPicker />
        </TabContent>
      </TabItem>
      <TabItem>
        <TabLabel>Swatches</TabLabel>
        <TabContent className="">
          <Swatches />
        </TabContent>
      </TabItem>
    </Tabs>
  );
}
