import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '@/contexts/AuthContext';
import { getAuthToken } from '@/utils/auth';
import { toast } from 'sonner';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { Textarea } from '@/components/ui/textarea';

const API = process.env.REACT_APP_BACKEND_URL;

const SANCTION_TYPES = [
  { value: 'warning', label: 'Avertissement' },
  { value: 'ban_1h', label: 'Ban 1 heure' },
  { value: 'ban_24h', label: 'Ban 24 heures' },
  { value: 'ban_30d', label: 'Ban 30 jours' },
  { value: 'ban_permanent', label: 'Ban permanent' },
];

const STATUS_COLORS = {
  pending: 'bg-yellow-500/20 text-yellow-400 border-yellow-500/30',
  actioned: 'bg-red-500/20 text-red-400 border-red-500/30',
  dismissed: 'bg-zinc-500/20 text-zinc-400 border-zinc-500/30',
};

const SANCTION_COLORS = {
  warning: 'bg-yellow-500/20 text-yellow-400',
  ban_1h: 'bg-orange-500/20 text-orange-400',
  ban_24h: 'bg-red-500/20 text-red-400',
  ban_30d: 'bg-red-600/20 text-red-500',
  ban_permanent: 'bg-red-800/20 text-red-300',
};

function timeAgo(dateStr) {
  const now = new Date();
  const date = new Date(dateStr);
  const diff = Math.floor((now - date) / 1000);
  if (diff < 60) return 'a l\'instant';
  if (diff < 3600) return `il y a ${Math.floor(diff / 60)}min`;
  if (diff < 86400) return `il y a ${Math.floor(diff / 3600)}h`;
  return `il y a ${Math.floor(diff / 86400)}j`;
}

function formatDate(dateStr) {
  if (!dateStr) return '-';
  return new Date(dateStr).toLocaleString('fr-FR', {
    day: '2-digit', month: '2-digit', year: 'numeric',
    hour: '2-digit', minute: '2-digit',
  });
}

export default function AdminDashboard() {
  const navigate = useNavigate();
  const { user } = useAuth();
  const [stats, setStats] = useState(null);
  const [reports, setReports] = useState([]);
  const [reportsTotal, setReportsTotal] = useState(0);
  const [reportsPage, setReportsPage] = useState(1);
  const [reportsFilter, setReportsFilter] = useState('pending');
  const [sanctions, setSanctions] = useState([]);
  const [sanctionsTotal, setSanctionsTotal] = useState(0);
  const [sanctionsPage, setSanctionsPage] = useState(1);
  const [auditLog, setAuditLog] = useState([]);
  const [auditTotal, setAuditTotal] = useState(0);
  const [auditPage, setAuditPage] = useState(1);
  const [loading, setLoading] = useState(true);

  // Review dialog state
  const [reviewDialog, setReviewDialog] = useState(false);
  const [selectedReport, setSelectedReport] = useState(null);
  const [reviewAction, setReviewAction] = useState('');
  const [sanctionType, setSanctionType] = useState('');
  const [reviewReason, setReviewReason] = useState('');
  const [submitting, setSubmitting] = useState(false);

  const headers = useCallback(() => ({
    'Authorization': `Bearer ${getAuthToken()}`,
    'Content-Type': 'application/json',
  }), []);

  // Fetch stats
  const fetchStats = useCallback(async () => {
    try {
      const res = await fetch(`${API}/api/admin/stats`, { headers: headers() });
      if (res.ok) {
        const json = await res.json();
        setStats(json.data || json);
      } else if (res.status === 403) {
        toast.error('Acces refuse - compte non admin');
        navigate('/live-prematch');
      }
    } catch (err) {
      console.error('Failed to fetch stats:', err);
    }
  }, [headers, navigate]);

  // Fetch reports
  const fetchReports = useCallback(async () => {
    try {
      const params = new URLSearchParams({ page: reportsPage, per_page: 10 });
      if (reportsFilter) params.set('status', reportsFilter);
      const res = await fetch(`${API}/api/admin/reports?${params}`, { headers: headers() });
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        setReports(data.items || []);
        setReportsTotal(data.total || 0);
      }
    } catch (err) {
      console.error('Failed to fetch reports:', err);
    }
  }, [headers, reportsPage, reportsFilter]);

  // Fetch active sanctions
  const fetchSanctions = useCallback(async () => {
    try {
      const params = new URLSearchParams({ page: sanctionsPage, per_page: 10 });
      const res = await fetch(`${API}/api/admin/sanctions?${params}`, { headers: headers() });
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        setSanctions(data.items || []);
        setSanctionsTotal(data.total || 0);
      }
    } catch (err) {
      console.error('Failed to fetch sanctions:', err);
    }
  }, [headers, sanctionsPage]);

  // Fetch audit log
  const fetchAuditLog = useCallback(async () => {
    try {
      const params = new URLSearchParams({ page: auditPage, per_page: 10 });
      const res = await fetch(`${API}/api/admin/audit-log?${params}`, { headers: headers() });
      if (res.ok) {
        const json = await res.json();
        const data = json.data || json;
        setAuditLog(data.items || []);
        setAuditTotal(data.total || 0);
      }
    } catch (err) {
      console.error('Failed to fetch audit log:', err);
    }
  }, [headers, auditPage]);

  // Initial load
  useEffect(() => {
    const loadAll = async () => {
      setLoading(true);
      await Promise.all([fetchStats(), fetchReports(), fetchSanctions(), fetchAuditLog()]);
      setLoading(false);
    };
    loadAll();
  }, [fetchStats, fetchReports, fetchSanctions, fetchAuditLog]);

  // Review report
  const handleReview = async () => {
    if (!selectedReport || !reviewAction) return;
    if (reviewAction === 'actioned' && !sanctionType) {
      toast.error('Selectionnez un type de sanction');
      return;
    }
    setSubmitting(true);
    try {
      const body = {
        status: reviewAction,
        ...(reviewAction === 'actioned' && { sanction_type: sanctionType }),
        ...(reviewReason && { reason: reviewReason }),
      };
      const res = await fetch(`${API}/api/admin/reports/${selectedReport.id}/review`, {
        method: 'PUT',
        headers: headers(),
        body: JSON.stringify(body),
      });
      if (res.ok) {
        toast.success(reviewAction === 'actioned' ? 'Sanction appliquee' : 'Signalement rejete');
        setReviewDialog(false);
        setSelectedReport(null);
        setReviewAction('');
        setSanctionType('');
        setReviewReason('');
        fetchReports();
        fetchStats();
        fetchSanctions();
        fetchAuditLog();
      } else {
        const err = await res.json();
        toast.error(err.error?.message || 'Erreur lors du traitement');
      }
    } catch (err) {
      toast.error('Erreur reseau');
    } finally {
      setSubmitting(false);
    }
  };

  // Lift sanction
  const handleLiftSanction = async (sanction) => {
    try {
      const res = await fetch(
        `${API}/api/admin/users/${sanction.user_id}/sanction/${sanction.id}`,
        { method: 'DELETE', headers: headers() }
      );
      if (res.ok) {
        toast.success('Sanction levee');
        fetchSanctions();
        fetchStats();
        fetchAuditLog();
      } else {
        toast.error('Erreur lors de la levee');
      }
    } catch (err) {
      toast.error('Erreur reseau');
    }
  };

  if (loading) {
    return (
      <div className="min-h-screen bg-[#050505] flex items-center justify-center">
        <div className="animate-spin rounded-full h-16 w-16 border-t-2 border-b-2 border-orange-500" />
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-[#050505] text-white">
      {/* Header */}
      <div className="border-b border-zinc-800 px-6 py-4 flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-full bg-orange-500 flex items-center justify-center text-sm font-bold">B</div>
          <h1 className="text-lg font-semibold">Brozr Admin</h1>
          <Badge variant="outline" className="text-orange-400 border-orange-500/30">
            {user?.display_name || 'Admin'}
          </Badge>
        </div>
        <Button variant="ghost" size="sm" onClick={() => navigate('/live-prematch')}
          className="text-zinc-400 hover:text-white">
          Retour a l'app
        </Button>
      </div>

      <div className="max-w-7xl mx-auto p-6 space-y-6">
        {/* Stats Cards */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Card className="bg-zinc-900 border-zinc-800">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm text-zinc-400">Signalements en attente</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-3xl font-bold text-yellow-400">{stats?.pending_reports ?? 0}</p>
            </CardContent>
          </Card>
          <Card className="bg-zinc-900 border-zinc-800">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm text-zinc-400">Sanctions actives</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-3xl font-bold text-red-400">{stats?.active_sanctions ?? 0}</p>
            </CardContent>
          </Card>
          <Card className="bg-zinc-900 border-zinc-800">
            <CardHeader className="pb-2">
              <CardTitle className="text-sm text-zinc-400">Signalements aujourd'hui</CardTitle>
            </CardHeader>
            <CardContent>
              <p className="text-3xl font-bold text-zinc-300">{stats?.reports_today ?? 0}</p>
            </CardContent>
          </Card>
        </div>

        {/* Tabs */}
        <Tabs defaultValue="reports" className="space-y-4">
          <TabsList className="bg-zinc-900 border border-zinc-800">
            <TabsTrigger value="reports" className="data-[state=active]:bg-zinc-700">
              Signalements {reportsTotal > 0 && `(${reportsTotal})`}
            </TabsTrigger>
            <TabsTrigger value="sanctions" className="data-[state=active]:bg-zinc-700">
              Sanctions {sanctionsTotal > 0 && `(${sanctionsTotal})`}
            </TabsTrigger>
            <TabsTrigger value="audit" className="data-[state=active]:bg-zinc-700">
              Journal d'audit
            </TabsTrigger>
          </TabsList>

          {/* Reports Tab */}
          <TabsContent value="reports" className="space-y-4">
            {/* Filter */}
            <div className="flex gap-2">
              {['pending', 'actioned', 'dismissed', ''].map((s) => (
                <Button
                  key={s || 'all'}
                  variant={reportsFilter === s ? 'default' : 'outline'}
                  size="sm"
                  onClick={() => { setReportsFilter(s); setReportsPage(1); }}
                  className={reportsFilter === s ? 'bg-orange-600 hover:bg-orange-700' : 'border-zinc-700 text-zinc-400'}
                >
                  {s === 'pending' ? 'En attente' : s === 'actioned' ? 'Traites' : s === 'dismissed' ? 'Rejetes' : 'Tous'}
                </Button>
              ))}
            </div>

            <Card className="bg-zinc-900 border-zinc-800">
              <Table>
                <TableHeader>
                  <TableRow className="border-zinc-800 hover:bg-transparent">
                    <TableHead className="text-zinc-400">Date</TableHead>
                    <TableHead className="text-zinc-400">Signaleur</TableHead>
                    <TableHead className="text-zinc-400">Signale</TableHead>
                    <TableHead className="text-zinc-400">Type</TableHead>
                    <TableHead className="text-zinc-400">Motif</TableHead>
                    <TableHead className="text-zinc-400">Statut</TableHead>
                    <TableHead className="text-zinc-400">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {reports.length === 0 ? (
                    <TableRow className="border-zinc-800">
                      <TableCell colSpan={7} className="text-center text-zinc-500 py-8">
                        Aucun signalement
                      </TableCell>
                    </TableRow>
                  ) : reports.map((report) => (
                    <TableRow key={report.id} className="border-zinc-800 hover:bg-zinc-800/50">
                      <TableCell className="text-zinc-400 text-xs">{timeAgo(report.created_at)}</TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">{report.reporter_id?.slice(0, 8)}...</TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">{report.reported_id?.slice(0, 8)}...</TableCell>
                      <TableCell>
                        <Badge variant="outline" className="text-xs border-zinc-600 text-zinc-300">
                          {report.report_type}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-sm text-zinc-300 max-w-[200px] truncate">{report.reason}</TableCell>
                      <TableCell>
                        <Badge className={`text-xs ${STATUS_COLORS[report.status] || ''}`}>
                          {report.status === 'pending' ? 'En attente' : report.status === 'actioned' ? 'Traite' : 'Rejete'}
                        </Badge>
                      </TableCell>
                      <TableCell>
                        {report.status === 'pending' && (
                          <Button
                            size="sm"
                            variant="outline"
                            className="border-orange-500/30 text-orange-400 hover:bg-orange-500/10 text-xs"
                            onClick={() => {
                              setSelectedReport(report);
                              setReviewDialog(true);
                            }}
                          >
                            Traiter
                          </Button>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </Card>

            {/* Pagination */}
            {reportsTotal > 10 && (
              <div className="flex justify-center gap-2">
                <Button size="sm" variant="outline" disabled={reportsPage <= 1}
                  onClick={() => setReportsPage(p => p - 1)} className="border-zinc-700 text-zinc-400">
                  Precedent
                </Button>
                <span className="text-sm text-zinc-400 py-2">
                  Page {reportsPage} / {Math.ceil(reportsTotal / 10)}
                </span>
                <Button size="sm" variant="outline" disabled={reportsPage >= Math.ceil(reportsTotal / 10)}
                  onClick={() => setReportsPage(p => p + 1)} className="border-zinc-700 text-zinc-400">
                  Suivant
                </Button>
              </div>
            )}
          </TabsContent>

          {/* Sanctions Tab */}
          <TabsContent value="sanctions" className="space-y-4">
            <Card className="bg-zinc-900 border-zinc-800">
              <Table>
                <TableHeader>
                  <TableRow className="border-zinc-800 hover:bg-transparent">
                    <TableHead className="text-zinc-400">Date</TableHead>
                    <TableHead className="text-zinc-400">Utilisateur</TableHead>
                    <TableHead className="text-zinc-400">Type</TableHead>
                    <TableHead className="text-zinc-400">Raison</TableHead>
                    <TableHead className="text-zinc-400">Expire</TableHead>
                    <TableHead className="text-zinc-400">Action</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {sanctions.length === 0 ? (
                    <TableRow className="border-zinc-800">
                      <TableCell colSpan={6} className="text-center text-zinc-500 py-8">
                        Aucune sanction active
                      </TableCell>
                    </TableRow>
                  ) : sanctions.map((sanction) => (
                    <TableRow key={sanction.id} className="border-zinc-800 hover:bg-zinc-800/50">
                      <TableCell className="text-zinc-400 text-xs">{timeAgo(sanction.created_at)}</TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">{sanction.user_id?.slice(0, 8)}...</TableCell>
                      <TableCell>
                        <Badge className={`text-xs ${SANCTION_COLORS[sanction.sanction_type] || ''}`}>
                          {SANCTION_TYPES.find(s => s.value === sanction.sanction_type)?.label || sanction.sanction_type}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-sm text-zinc-300 max-w-[200px] truncate">{sanction.reason}</TableCell>
                      <TableCell className="text-xs text-zinc-400">
                        {sanction.expires_at ? formatDate(sanction.expires_at) : 'Permanent'}
                      </TableCell>
                      <TableCell>
                        <Button
                          size="sm"
                          variant="outline"
                          className="border-green-500/30 text-green-400 hover:bg-green-500/10 text-xs"
                          onClick={() => handleLiftSanction(sanction)}
                        >
                          Lever
                        </Button>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </Card>

            {sanctionsTotal > 10 && (
              <div className="flex justify-center gap-2">
                <Button size="sm" variant="outline" disabled={sanctionsPage <= 1}
                  onClick={() => setSanctionsPage(p => p - 1)} className="border-zinc-700 text-zinc-400">
                  Precedent
                </Button>
                <span className="text-sm text-zinc-400 py-2">
                  Page {sanctionsPage} / {Math.ceil(sanctionsTotal / 10)}
                </span>
                <Button size="sm" variant="outline" disabled={sanctionsPage >= Math.ceil(sanctionsTotal / 10)}
                  onClick={() => setSanctionsPage(p => p + 1)} className="border-zinc-700 text-zinc-400">
                  Suivant
                </Button>
              </div>
            )}
          </TabsContent>

          {/* Audit Log Tab */}
          <TabsContent value="audit" className="space-y-4">
            <Card className="bg-zinc-900 border-zinc-800">
              <Table>
                <TableHeader>
                  <TableRow className="border-zinc-800 hover:bg-transparent">
                    <TableHead className="text-zinc-400">Date</TableHead>
                    <TableHead className="text-zinc-400">Admin</TableHead>
                    <TableHead className="text-zinc-400">Action</TableHead>
                    <TableHead className="text-zinc-400">Utilisateur cible</TableHead>
                    <TableHead className="text-zinc-400">Details</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {auditLog.length === 0 ? (
                    <TableRow className="border-zinc-800">
                      <TableCell colSpan={5} className="text-center text-zinc-500 py-8">
                        Aucune action enregistree
                      </TableCell>
                    </TableRow>
                  ) : auditLog.map((entry) => (
                    <TableRow key={entry.id} className="border-zinc-800 hover:bg-zinc-800/50">
                      <TableCell className="text-zinc-400 text-xs">{formatDate(entry.created_at)}</TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">{entry.admin_id?.slice(0, 8)}...</TableCell>
                      <TableCell>
                        <Badge variant="outline" className="text-xs border-zinc-600 text-zinc-300">
                          {entry.action?.replace(/_/g, ' ')}
                        </Badge>
                      </TableCell>
                      <TableCell className="font-mono text-xs text-zinc-300">
                        {entry.target_user_id ? `${entry.target_user_id.slice(0, 8)}...` : '-'}
                      </TableCell>
                      <TableCell className="text-xs text-zinc-400 max-w-[300px] truncate">
                        {entry.details ? JSON.stringify(entry.details) : '-'}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
            </Card>

            {auditTotal > 10 && (
              <div className="flex justify-center gap-2">
                <Button size="sm" variant="outline" disabled={auditPage <= 1}
                  onClick={() => setAuditPage(p => p - 1)} className="border-zinc-700 text-zinc-400">
                  Precedent
                </Button>
                <span className="text-sm text-zinc-400 py-2">
                  Page {auditPage} / {Math.ceil(auditTotal / 10)}
                </span>
                <Button size="sm" variant="outline" disabled={auditPage >= Math.ceil(auditTotal / 10)}
                  onClick={() => setAuditPage(p => p + 1)} className="border-zinc-700 text-zinc-400">
                  Suivant
                </Button>
              </div>
            )}
          </TabsContent>
        </Tabs>
      </div>

      {/* Review Dialog */}
      <Dialog open={reviewDialog} onOpenChange={setReviewDialog}>
        <DialogContent className="bg-zinc-900 border-zinc-800 text-white max-w-md">
          <DialogHeader>
            <DialogTitle>Traiter le signalement</DialogTitle>
          </DialogHeader>

          {selectedReport && (
            <div className="space-y-4">
              <div className="bg-zinc-800 rounded-lg p-3 space-y-2 text-sm">
                <div className="flex justify-between">
                  <span className="text-zinc-400">Type :</span>
                  <span>{selectedReport.report_type}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-zinc-400">Motif :</span>
                  <span className="text-right max-w-[200px]">{selectedReport.reason}</span>
                </div>
                {selectedReport.context && (
                  <div>
                    <span className="text-zinc-400">Contexte :</span>
                    <p className="mt-1 text-zinc-300">{selectedReport.context}</p>
                  </div>
                )}
                <div className="flex justify-between">
                  <span className="text-zinc-400">Signale :</span>
                  <span className="font-mono text-xs">{selectedReport.reported_id}</span>
                </div>
              </div>

              {/* Action choice */}
              <div className="flex gap-2">
                <Button
                  variant={reviewAction === 'actioned' ? 'default' : 'outline'}
                  size="sm"
                  className={reviewAction === 'actioned' ? 'bg-red-600 hover:bg-red-700' : 'border-zinc-700 text-zinc-400'}
                  onClick={() => setReviewAction('actioned')}
                >
                  Sanctionner
                </Button>
                <Button
                  variant={reviewAction === 'dismissed' ? 'default' : 'outline'}
                  size="sm"
                  className={reviewAction === 'dismissed' ? 'bg-zinc-600 hover:bg-zinc-700' : 'border-zinc-700 text-zinc-400'}
                  onClick={() => setReviewAction('dismissed')}
                >
                  Rejeter
                </Button>
              </div>

              {/* Sanction type selector */}
              {reviewAction === 'actioned' && (
                <Select value={sanctionType} onValueChange={setSanctionType}>
                  <SelectTrigger className="bg-zinc-800 border-zinc-700">
                    <SelectValue placeholder="Type de sanction..." />
                  </SelectTrigger>
                  <SelectContent className="bg-zinc-800 border-zinc-700">
                    {SANCTION_TYPES.map((s) => (
                      <SelectItem key={s.value} value={s.value} className="text-white hover:bg-zinc-700">
                        {s.label}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              )}

              {/* Optional reason */}
              <Textarea
                placeholder="Raison (optionnel)..."
                value={reviewReason}
                onChange={(e) => setReviewReason(e.target.value)}
                className="bg-zinc-800 border-zinc-700 text-white"
                rows={2}
              />
            </div>
          )}

          <DialogFooter>
            <Button variant="ghost" onClick={() => setReviewDialog(false)} className="text-zinc-400">
              Annuler
            </Button>
            <Button
              onClick={handleReview}
              disabled={!reviewAction || submitting}
              className={reviewAction === 'actioned' ? 'bg-red-600 hover:bg-red-700' : 'bg-zinc-600 hover:bg-zinc-700'}
            >
              {submitting ? 'En cours...' : 'Confirmer'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
