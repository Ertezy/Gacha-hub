import type { HubData } from "../types";

interface Props {
  hub: HubData | null;
  contentIds: string[];
}

/** Заглушка: настоящую панель собирает задача 12. */
export default function SidePanel(_props: Props) {
  return <aside className="panel" />;
}
