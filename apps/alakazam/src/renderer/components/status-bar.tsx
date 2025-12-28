import { Option } from '@/ui/option';
import { Select } from '@/ui/select';

export function StatusBar() {
  return (
    <div className="bg-dark flex h-8 w-full items-center border-t border-white/30 px-4 text-sm">
      <Select size="sm" className="w-20">
        <Option value="here">Here</Option>
      </Select>
    </div>
  );
}
