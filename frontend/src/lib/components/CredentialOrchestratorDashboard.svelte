<script lang="ts">
    import ConsumerProfilesPanel from './ConsumerProfilesPanel.svelte';
    import AccessPoliciesPanel from './AccessPoliciesPanel.svelte';
    import ProxyAuditPanel from './ProxyAuditPanel.svelte';

    let activeTab: 'profiles' | 'policies' | 'audit' = 'profiles';
</script>

<div class="space-y-6">
    <div>
        <h2 class="text-lg font-bold text-white mb-1">Credential Orchestrator</h2>
        <p class="text-xs text-slate-500">Manage AI consumer identities, access policies, and proxy audit trail.</p>
    </div>

    <!-- Tab navigation -->
    <div class="flex gap-1 p-1 rounded-xl bg-slate-950/50 border border-slate-800 w-fit">
        <button
            on:click={() => activeTab = 'profiles'}
            class="px-4 py-1.5 rounded-lg text-xs font-bold transition-all {activeTab === 'profiles' ? 'bg-sky-600 text-white shadow' : 'text-slate-400 hover:text-white hover:bg-slate-800'}"
        >
            Consumer Profiles
        </button>
        <button
            on:click={() => activeTab = 'policies'}
            class="px-4 py-1.5 rounded-lg text-xs font-bold transition-all {activeTab === 'policies' ? 'bg-sky-600 text-white shadow' : 'text-slate-400 hover:text-white hover:bg-slate-800'}"
        >
            Access Policies
        </button>
        <button
            on:click={() => activeTab = 'audit'}
            class="px-4 py-1.5 rounded-lg text-xs font-bold transition-all {activeTab === 'audit' ? 'bg-sky-600 text-white shadow' : 'text-slate-400 hover:text-white hover:bg-slate-800'}"
        >
            Proxy Audit
        </button>
    </div>

    <!-- Tab content -->
    {#if activeTab === 'profiles'}
        <ConsumerProfilesPanel />
    {:else if activeTab === 'policies'}
        <AccessPoliciesPanel />
    {:else if activeTab === 'audit'}
        <ProxyAuditPanel />
    {/if}
</div>
