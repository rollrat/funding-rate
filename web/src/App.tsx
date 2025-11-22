import { useState, useEffect } from 'react';
import SnapshotTable from './components/SnapshotTable';
import type { PerpSnapshot } from './types';

function App() {
    const [snapshots, setSnapshots] = useState<PerpSnapshot[]>([]);
    const [loading, setLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const fetchSnapshots = async () => {
        try {
            const response = await fetch('http://localhost:12090/snapshots');
            if (!response.ok) {
                throw new Error('데이터를 가져오는데 실패했습니다');
            }
            const data: PerpSnapshot[] = await response.json();
            setSnapshots(data);
            setError(null);
        } catch (err) {
            setError(err instanceof Error ? err.message : '알 수 없는 오류가 발생했습니다');
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchSnapshots();

        // 10초마다 자동 새로고침
        const interval = setInterval(fetchSnapshots, 10000);

        return () => clearInterval(interval);
    }, []);

    if (loading) {
        return (
            <div className="min-h-screen bg-slate-900 flex items-center justify-center">
                <div className="text-slate-300 text-lg">데이터를 불러오는 중...</div>
            </div>
        );
    }

    if (error) {
        return (
            <div className="min-h-screen bg-slate-900 flex items-center justify-center">
                <div className="text-red-400 text-lg">오류: {error}</div>
            </div>
        );
    }

    return (
        <div className="min-h-screen bg-slate-900 py-8 px-4">
            <div className="max-w-7xl mx-auto">
                <h1 className="text-3xl font-bold text-slate-100 mb-6">Perp Scanner</h1>
                <SnapshotTable snapshots={snapshots} />
            </div>
        </div>
    );
}

export default App;

