import React, { use, useEffect, useState } from 'react';
import Button from '@mui/material/Button';
import Paper from '@mui/material/Paper';
import Box from '@mui/material/Box';
import { TableBody, TableCell, TableHead, TableRow, TextField, Typography } from '@mui/material';
import Table from '@mui/material/Table';
import { useWalletContext, WalletContextProvider } from './utils/wallet.context';

export const Delegations = () => {
  const { delegations, doDelegate, delegationLoader, unDelegateAll, unDelegateAllLoading, log } = useWalletContext();

  const [delegationNodeId, setDelegationNodeId] = useState<string>();
  const [amountToBeDelegated, setAmountToBeDelegated] = useState<string>();

  const cleanFields = () => {
    setDelegationNodeId('');
    setAmountToBeDelegated('');
  };

  useEffect(() => {
    return () => {
      cleanFields();
    };
  }, []);

  return (
    <Box>
      <Paper style={{ marginTop: '1rem', padding: '1rem' }}>
        <Box padding={3}>
          <Typography variant="h6">Delegations</Typography>
          <Box marginY={3}>
            <Box marginY={3} display="flex" flexDirection="column">
              <Typography marginBottom={3} variant="body1">
                Make a delegation
              </Typography>
              <TextField
                type="text"
                placeholder="Mixnode ID"
                onChange={(e) => setDelegationNodeId(e.target.value)}
                size="small"
              />
              <Box marginTop={3} display="flex" justifyContent="space-between">
                <TextField
                  type="text"
                  placeholder="Amount"
                  onChange={(e) => setAmountToBeDelegated(e.target.value)}
                  size="small"
                />
                <Button
                  variant="outlined"
                  onClick={() => {
                    doDelegate({ mixId: delegationNodeId, amount: amountToBeDelegated });
                    cleanFields();
                  }}
                  disabled={delegationLoader}
                >
                  {delegationLoader ? 'Delegation in process...' : 'Delegate'}
                </Button>
              </Box>
            </Box>
          </Box>
          <Box marginTop={3}>
            <Typography variant="body1">Your delegations:</Typography>
            <Box marginBottom={3} display="flex" flexDirection="column">
              {!delegations?.delegations?.length ? (
                <Typography variant="body2">You do not have delegations</Typography>
              ) : (
                <Box overflow='auto'>
                  <Table size="small">
                    <TableHead>
                      <TableRow>
                        <TableCell>MixId</TableCell>
                        <TableCell>Owner</TableCell>
                        <TableCell>Amount</TableCell>
                        <TableCell>Cumulative Reward Ratio</TableCell>
                      </TableRow>
                    </TableHead>
                    <TableBody>
                      {delegations?.delegations.map((delegation: any) => (
                        <TableRow key={delegation.mix_id}>
                          <TableCell>{delegation.mix_id}</TableCell>
                          <TableCell>{delegation.owner}</TableCell>
                          <TableCell>{delegation.amount.amount}</TableCell>
                          <TableCell>{delegation.cumulative_reward_ratio}</TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </Box>
              )}
            </Box>
            {delegations && (
              <Box marginBottom={3}>
                <Button variant="outlined" onClick={() => unDelegateAll()} disabled={unDelegateAllLoading}>
                  {unDelegateAllLoading ? 'Undelegating...' : 'Undelegate All'}
                </Button>
              </Box>
            )}
          </Box>
        </Box>
      </Paper>
      {log.length > 0 && (
        <Box marginTop={3}>
          <Typography variant="h5">Transaction Logs:</Typography>
          {log}
        </Box>
      )}
    </Box>
  );
};
